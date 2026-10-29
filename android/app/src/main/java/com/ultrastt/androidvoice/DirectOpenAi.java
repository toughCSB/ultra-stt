package com.ultrastt.androidvoice;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.function.Consumer;

import okhttp3.Call;
import okhttp3.MediaType;
import okhttp3.MultipartBody;
import okhttp3.OkHttpClient;
import okhttp3.Request;
import okhttp3.RequestBody;
import okhttp3.Response;

final class DirectOpenAi {
    static final String TRANSCRIPTIONS = "https://api.openai.com/v1/audio/transcriptions";
    static final String RESPONSES = "https://api.openai.com/v1/responses";

    static ConnectionProbe.Result checkConnection(OkHttpClient client, String key) {
        if (key == null || key.trim().isEmpty()) {
            return new ConnectionProbe.Result(false, "OpenAI API 키를 먼저 저장해 주세요.");
        }
        OkHttpClient bounded = client.newBuilder().callTimeout(15, java.util.concurrent.TimeUnit.SECONDS).build();
        try {
            for (String model : new String[]{"gpt-transcribe", "gpt-6-luna"}) {
                Request request = new Request.Builder().url("https://api.openai.com/v1/models/" + model)
                        .header("Authorization", "Bearer " + key.trim()).get().build();
                try (Response response = bounded.newCall(request).execute()) {
                    if (response.code() != 200) {
                        return new ConnectionProbe.Result(false, "모델 조회 실패 (HTTP " + response.code()
                                + "). API 키·모델 조회 권한을 확인해 주세요.");
                    }
                    JSONObject value = new JSONObject(HttpTransport.read(response.body(), 65536));
                    if (!"model".equals(value.optString("object")) || value.optString("id").isEmpty()) {
                        return new ConnectionProbe.Result(false, "OpenAI 모델 조회 응답이 올바르지 않습니다.");
                    }
                }
            }
            return new ConnectionProbe.Result(true,
                    "연결 성공 · OpenAI 인증 및 전사·다듬기 모델 조회 확인. 실제 음성·생성 요청은 실행하지 않았습니다.");
        } catch (Exception failure) {
            return new ConnectionProbe.Result(false, "OpenAI 연결 확인에 실패했습니다. 인터넷 연결을 확인해 주세요.");
        }
    }

    static Request transcriptionRequest(String key, byte[] wav) {
        MultipartBody body = new MultipartBody.Builder().setType(MultipartBody.FORM)
                .addFormDataPart("model", "gpt-transcribe")
                .addFormDataPart("response_format", "json")
                .addFormDataPart("languages[]", "ko")
                .addFormDataPart("file", "voice.wav", RequestBody.create(wav, MediaType.get("audio/wav")))
                .build();
        return new Request.Builder().url(TRANSCRIPTIONS).header("Authorization", "Bearer " + key)
                .header("Accept", "application/json").post(body).build();
    }

    static Request polishRequest(String key, String raw, String mode) throws Exception {
        JSONObject payload = new JSONObject().put("model", "gpt-6-luna")
                .put("reasoning", new JSONObject().put("effort", reasoningEffort(mode)))
                .put("service_tier", "priority").put("store", false).put("max_output_tokens", 8192)
                .put("input", new JSONArray()
                        .put(new JSONObject().put("role", "system").put("content", PolishPrompt.forMode(mode)))
                        .put(new JSONObject().put("role", "user").put("content", "<transcription>\n" + raw
                                + "\n</transcription>")));
        return new Request.Builder().url(RESPONSES).header("Authorization", "Bearer " + key)
                .header("Accept", "application/json")
                .post(RequestBody.create(payload.toString().getBytes(StandardCharsets.UTF_8),
                        MediaType.get("application/json; charset=utf-8"))).build();
    }

    static String reasoningEffort(String mode) {
        return "structured".equals(mode) || "professional".equals(mode) ? "medium" : "low";
    }

    static String process(OkHttpClient client, String key, byte[] wav, String mode, boolean polish,
            AtomicBoolean cancelled, Consumer<Call> activeCall) throws Exception {
        return process(client, key, wav, mode, polish, cancelled, activeCall, notice -> {});
    }

    static String process(OkHttpClient client, String key, byte[] wav, String mode, boolean polish,
            AtomicBoolean cancelled, Consumer<Call> activeCall, Consumer<String> warning) throws Exception {
        String raw = parseTranscription(execute(client, transcriptionRequest(key, wav), 1024 * 1024,
                cancelled, activeCall));
        if (!polish || cancelled.get()) {
            return raw;
        }
        try {
            return parsePolish(execute(client, polishRequest(key, raw, mode), 256 * 1024,
                    cancelled, activeCall));
        } catch (Exception error) {
            if (!cancelled.get()) {
                warning.accept("AI 다듬기에 실패하여 전사 원문을 그대로 입력합니다.");
            }
            return raw;
        }
    }

    private static String execute(OkHttpClient client, Request request, int limit,
            AtomicBoolean cancelled, Consumer<Call> activeCall) throws Exception {
        if (cancelled.get()) {
            throw new IOException("Cancelled");
        }
        Call call = client.newCall(request);
        activeCall.accept(call);
        if (cancelled.get()) {
            call.cancel();
            throw new IOException("Cancelled");
        }
        try (Response response = call.execute()) {
            if (response.code() == 401 || response.code() == 403) {
                throw new DirectError("OpenAI API 키를 확인해 주세요.");
            }
            if (response.code() != 200) {
                throw new DirectError("OpenAI 처리 오류 (HTTP " + response.code() + "). 잠시 후 다시 시도해 주세요.");
            }
            return HttpTransport.read(response.body(), limit);
        } finally {
            activeCall.accept(null);
        }
    }

    static String parseTranscription(String json) throws Exception {
        JSONObject result = new JSONObject(json);
        if (result.has("error") && !result.isNull("error")) {
            throw new DirectError("OpenAI 음성 처리가 실패했습니다. 잠시 후 다시 시도해 주세요.");
        }
        Object value = result.opt("text");
        if (!(value instanceof String) || ((String) value).trim().isEmpty()) {
            throw new DirectError("인식된 문장이 없습니다. 마이크를 확인하고 다시 말해 주세요.");
        }
        return ((String) value).trim();
    }

    static String parsePolish(String json) throws Exception {
        JSONObject result = new JSONObject(json);
        if (!"completed".equals(result.optString("status"))
                || result.has("error") && !result.isNull("error")
                || result.has("incomplete_details") && !result.isNull("incomplete_details")) {
            throw new IOException("Polish failed");
        }
        JSONArray output = result.getJSONArray("output");
        StringBuilder text = new StringBuilder();
        int messages = 0;
        for (int index = 0; index < output.length(); index++) {
            JSONObject item = output.getJSONObject(index);
            if ("reasoning".equals(item.optString("type"))) {
                continue;
            }
            if (!"message".equals(item.optString("type"))) {
                throw new IOException("Unexpected polish output");
            }
            if (!"assistant".equals(item.optString("role"))
                    || !"completed".equals(item.optString("status")) || ++messages != 1) {
                throw new IOException("Incomplete polish message");
            }
            JSONArray content = item.getJSONArray("content");
            for (int part = 0; part < content.length(); part++) {
                JSONObject segment = content.getJSONObject(part);
                if (!"output_text".equals(segment.optString("type"))
                        || !(segment.opt("text") instanceof String)) {
                    throw new IOException("Non-text polish output");
                }
                text.append(segment.getString("text"));
            }
        }
        if (messages != 1 || text.toString().trim().isEmpty()) {
            throw new IOException("Empty polish");
        }
        return text.toString().trim();
    }

    static final class DirectError extends Exception {
        DirectError(String message) {
            super(message);
        }
    }

    private DirectOpenAi() {}
}
