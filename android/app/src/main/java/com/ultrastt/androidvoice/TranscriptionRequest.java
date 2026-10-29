package com.ultrastt.androidvoice;

import android.os.Handler;
import android.os.Looper;

import org.json.JSONObject;

import java.util.concurrent.atomic.AtomicBoolean;

import okhttp3.Call;
import okhttp3.MediaType;
import okhttp3.MultipartBody;
import okhttp3.Request;
import okhttp3.RequestBody;
import okhttp3.Response;

final class TranscriptionRequest implements Runnable {
    interface Listener {
        void onResult(String text);
        void onError(String message);
        default void onNotice(String message) {}
    }

    private final Handler main = new Handler(Looper.getMainLooper());
    private final AtomicBoolean cancelled = new AtomicBoolean();
    private final String endpoint;
    private final String engine;
    private final String address;
    private final String token;
    private final String mode;
    private final boolean polish;
    private final byte[] wav;
    private final Listener listener;
    private volatile Call call;

    TranscriptionRequest(String endpoint, String address, String token, String mode, boolean polish,
            byte[] wav, Listener listener) {
        this.engine = Engine.LEGACY;
        this.endpoint = Endpoint.normalize(endpoint);
        this.address = address;
        this.token = token;
        this.mode = Modes.IDS[Modes.index(mode)];
        this.polish = polish;
        this.wav = wav;
        this.listener = listener;
    }

    private TranscriptionRequest(String key, String mode, boolean polish, byte[] wav, Listener listener) {
        this.engine = Engine.OPENAI;
        this.endpoint = DirectOpenAi.TRANSCRIPTIONS;
        this.address = "";
        this.token = key;
        this.mode = Modes.IDS[Modes.index(mode)];
        this.polish = polish;
        this.wav = wav;
        this.listener = listener;
    }

    static TranscriptionRequest direct(String key, String mode, boolean polish, byte[] wav, Listener listener) {
        return new TranscriptionRequest(key, mode, polish, wav, listener);
    }

    void cancel() {
        cancelled.set(true);
        Call active = call;
        if (active != null) {
            active.cancel();
        }
    }

    @Override
    public void run() {
        if (Engine.OPENAI.equals(engine)) {
            runDirect();
            return;
        }
        try {
            if (cancelled.get()) {
                return;
            }
            MultipartBody body = new MultipartBody.Builder().setType(MultipartBody.FORM)
                    .addFormDataPart("model", "whisper-1")
                    .addFormDataPart("file", "voice.wav", RequestBody.create(wav, MediaType.get("audio/wav")))
                    .build();
            Request upload = new Request.Builder().url(endpoint)
                    .header("Authorization", "Bearer " + token).header("Accept", "application/json")
                    .header("x-ultra-stt-polish-style", mode)
                    .header("x-ultra-stt-polish-enabled", Boolean.toString(polish)).post(body).build();
            Call active = HttpTransport.client(endpoint, address, false).newCall(upload);
            call = active;
            if (cancelled.get()) {
                active.cancel();
                return;
            }
            try (Response response = active.execute()) {
                int status = response.code();
                if (status == 401 || status == 403) {
                    throw new VoiceError("인증에 실패했습니다. PC 연결 정보를 다시 받아 주세요.");
                }
                if (status != 200) {
                    throw new VoiceError("서버 응답 오류 (HTTP " + status + "). PC 앱 상태를 확인해 주세요.");
                }
                JSONObject result = new JSONObject(HttpTransport.read(response.body(), 1024 * 1024));
                if (result.has("error")) {
                    throw new VoiceError("인식된 문장이 없거나 음성 처리가 실패했습니다. 한 문장을 말하고 다시 시도해 주세요.");
                }
                String text = result.optString("text", "").trim();
                if (text.isEmpty()) {
                    throw new VoiceError("인식된 문장이 없습니다. 마이크를 확인하고 다시 말해 주세요.");
                }
                main.post(() -> {
                    if (!cancelled.get()) {
                        listener.onResult(text);
                    }
                });
            }
        } catch (Exception error) {
            String message;
            if (error instanceof VoiceError) {
                message = error.getMessage();
            } else if (error instanceof java.io.InterruptedIOException) {
                message = "서버 연결 또는 음성 처리 시간이 초과됐습니다. PC와 Tailscale 연결을 확인해 주세요.";
            } else if (error instanceof javax.net.ssl.SSLException) {
                message = "안전한 HTTPS 연결을 만들지 못했습니다. 서버 주소와 기기 날짜를 확인해 주세요.";
            } else if (error instanceof java.net.UnknownHostException) {
                message = "PC 주소를 찾지 못했습니다. 앱 전용 연결 정보를 다시 받아 주세요.";
            } else {
                message = "서버에 연결하지 못했습니다. 휴대폰의 Tailscale 연결과 서버 PC를 확인해 주세요.";
            }
            main.post(() -> {
                if (!cancelled.get()) {
                    listener.onError(message);
                }
            });
        } finally {
            call = null;
        }
    }

    private void runDirect() {
        try {
            java.util.concurrent.atomic.AtomicReference<String> warning = new java.util.concurrent.atomic.AtomicReference<>();
            String text = DirectOpenAi.process(HttpTransport.openAiClient(), token, wav, mode, polish,
                    cancelled, active -> call = active, warning::set);
            main.post(() -> {
                if (!cancelled.get()) {
                    if (warning.get() != null) {
                        listener.onNotice(warning.get());
                    }
                    listener.onResult(text);
                }
            });
        } catch (Exception error) {
            String message = error instanceof DirectOpenAi.DirectError ? error.getMessage()
                    : error instanceof java.io.InterruptedIOException
                    ? "OpenAI 음성 처리 시간이 초과됐습니다. 다시 시도해 주세요."
                    : "OpenAI에 연결하거나 응답을 읽지 못했습니다. 네트워크를 확인해 주세요.";
            main.post(() -> {
                if (!cancelled.get()) {
                    listener.onError(message);
                }
            });
        } finally {
            call = null;
        }
    }

    private static final class VoiceError extends Exception {
        VoiceError(String message) {
            super(message);
        }
    }
}
