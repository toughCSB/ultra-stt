package com.ultrastt.androidvoice;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.Context;
import android.content.SharedPreferences;
import android.os.Bundle;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.atomic.AtomicBoolean;

import okhttp3.OkHttpClient;
import okhttp3.Protocol;
import okhttp3.Request;
import okhttp3.Response;
import okhttp3.ResponseBody;
import okio.Buffer;

public final class DirectOpenAiChecks extends Instrumentation {
    private int checks;

    @Override
    public void onCreate(Bundle arguments) {
        super.onCreate(arguments);
        start();
    }

    @Override
    public void onStart() {
        Bundle result = new Bundle();
        int status = Activity.RESULT_CANCELED;
        try {
            Context testContext = new android.content.ContextWrapper(getTargetContext()) {
                @Override
                public Context getApplicationContext() { return this; }
                @Override
                public SharedPreferences getSharedPreferences(String name, int mode) {
                    return super.getSharedPreferences("qa-direct-" + name, mode);
                }
            };
            SharedPreferences store = testContext.getSharedPreferences("voice-settings", Context.MODE_PRIVATE);
            store.edit().clear().commit();
            store.edit().putString("endpoint", "https://sb.example.ts.net/v1/audio/transcriptions")
                    .putString("token-encrypted", "legacy-sentinel")
                    .putString("paired-address", "100.101.102.103").commit();
            Preferences preferences = new Preferences(testContext);
            check(Engine.LEGACY.equals(preferences.engine()), "Legacy engine is the default");
            preferences.saveOpenAiKey("test-only-key");
            preferences.engine(Engine.OPENAI);
            check(Engine.OPENAI.equals(preferences.engine()) && "test-only-key".equals(preferences.openAiKey()),
                    "Direct mode and encrypted key survive a preference reload");
            check(!store.getString("openai-key-encrypted", "").contains("test-only-key"),
                    "OpenAI key is not stored as plaintext");
            preferences.engine(Engine.LEGACY);
            check("legacy-sentinel".equals(store.getString("token-encrypted", ""))
                    && "100.101.102.103".equals(store.getString("paired-address", "")),
                    "Engine changes preserve legacy authentication and pairing");
            preferences.clearOpenAiKey();
            check(!preferences.hasOpenAiKey() && "legacy-sentinel".equals(store.getString("token-encrypted", "")),
                    "Clearing the direct key preserves legacy credentials");
            check(HttpTransport.openAiClient().dns() == okhttp3.Dns.SYSTEM
                    && !HttpTransport.openAiClient().followRedirects()
                    && !HttpTransport.openAiClient().followSslRedirects(),
                    "OpenAI transport uses system DNS and never follows redirects");
            List<Request> probes = new ArrayList<>();
            OkHttpClient probeClient = HttpTransport.openAiClient().newBuilder().addInterceptor(chain -> {
                probes.add(chain.request());
                return response(chain.request(), 200, "{\"id\":\"test-model\",\"object\":\"model\"}");
            }).build();
            check(DirectOpenAi.checkConnection(probeClient, "test-only-key").verified,
                    "Connection probe accepts authenticated model metadata");
            check(probes.size() == 2 && probes.stream().allMatch(request -> "GET".equals(request.method())
                    && request.url().encodedPath().startsWith("/v1/models/") && request.body() == null),
                    "Connection probes never upload audio or request paid generation");
            OkHttpClient denied = HttpTransport.openAiClient().newBuilder().addInterceptor(chain ->
                    response(chain.request(), 401, "{}")).build();
            check(!DirectOpenAi.checkConnection(denied, "invalid-test-key").verified,
                    "Rejected key cannot be reported as connected");
            check(!DirectOpenAi.checkConnection(probeClient, "").verified && probes.size() == 2,
                    "Missing key does not make a request");

            List<Request> requests = new ArrayList<>();
            OkHttpClient mocked = new OkHttpClient.Builder().followRedirects(false).followSslRedirects(false)
                    .addInterceptor(chain -> {
                        Request request = chain.request();
                        requests.add(request);
                        String body = request.url().encodedPath().endsWith("/audio/transcriptions")
                                ? "{\"text\":\"raw sentence\"}" : "{\"error\":{\"message\":\"mock failure\"}}";
                        return response(request, 200, body);
                    }).build();
            List<String> warnings = new ArrayList<>();
            String raw = DirectOpenAi.process(mocked, "test-only-key", new byte[]{1, 2, 3}, "structured",
                    true, new AtomicBoolean(), active -> {}, warnings::add);
            check("raw sentence".equals(raw), "Failed polish returns complete raw transcription");
            check(warnings.size() == 1 && warnings.get(0).contains("원문"),
                    "Raw fallback is explicitly surfaced to the caller");
            check(requests.size() == 2 && DirectOpenAi.TRANSCRIPTIONS.equals(requests.get(0).url().toString())
                    && DirectOpenAi.RESPONSES.equals(requests.get(1).url().toString()),
                    "Both calls use fixed OpenAI endpoints");
            for (Request request : requests) {
                check("Bearer test-only-key".equals(request.header("Authorization"))
                        && request.header("x-ultra-stt-polish-style") == null
                        && request.header("x-ultra-stt-polish-enabled") == null,
                        "Direct requests never carry SB authentication or control headers");
            }
            Buffer audioBody = new Buffer();
            requests.get(0).body().writeTo(audioBody);
            String multipart = audioBody.readUtf8();
            check(multipart.contains("name=\"languages[]\"") && multipart.contains("ko")
                    && !multipart.contains("name=\"language\"") && multipart.contains("gpt-transcribe"),
                    "Transcription uses plural Korean language hints");
            Buffer polishBody = new Buffer();
            requests.get(1).body().writeTo(polishBody);
            String json = polishBody.readUtf8();
            check(json.contains("\"reasoning\":{\"effort\":\"medium\"}")
                    && json.contains("\"service_tier\":\"priority\"")
                    && json.contains("\"store\":false") && json.contains("\"max_output_tokens\":8192")
                    && !json.contains("temperature"),
                    "Responses polish uses medium reasoning, priority, no storage and a bounded output");
            check("polished".equals(DirectOpenAi.parsePolish("{\"status\":\"completed\",\"error\":null,"
                    + "\"output\":[{\"type\":\"reasoning\"},{\"type\":\"message\",\"role\":\"assistant\","
                    + "\"status\":\"completed\",\"content\":[{\"type\":\"output_text\",\"text\":\"polished\"}]}]}")),
                    "Only a completed assistant output_text is accepted");
            for (String invalid : new String[]{
                    "{\"status\":\"incomplete\",\"output\":[{\"type\":\"message\",\"role\":\"assistant\",\"status\":\"completed\",\"content\":[{\"type\":\"output_text\",\"text\":\"partial\"}]}]}",
                    "{\"status\":\"completed\",\"output\":[{\"type\":\"message\",\"role\":\"assistant\",\"status\":\"incomplete\",\"content\":[{\"type\":\"output_text\",\"text\":\"partial\"}]}]}",
                    "{\"status\":\"completed\",\"output\":[{\"type\":\"message\",\"role\":\"assistant\",\"status\":\"completed\",\"content\":[{\"type\":\"refusal\",\"refusal\":\"no\"}]}]}"}) {
                boolean rejectedPolish = false;
                try {
                    DirectOpenAi.parsePolish(invalid);
                } catch (Exception expected) {
                    rejectedPolish = true;
                }
                check(rejectedPolish, "Incomplete or refusal polish falls back to raw text");
            }
            requests.clear();
            AtomicBoolean cancelled = new AtomicBoolean(true);
            boolean rejected = false;
            try {
                DirectOpenAi.process(mocked, "test-only-key", new byte[]{1}, "minimal", true,
                        cancelled, active -> {});
            } catch (IOException expected) {
                rejected = true;
            }
            check(rejected && requests.isEmpty(), "Cancellation before upload sends nothing");
            AtomicBoolean cancelledAfterUpload = new AtomicBoolean();
            OkHttpClient cancelling = new OkHttpClient.Builder().addInterceptor(chain -> {
                requests.add(chain.request());
                cancelledAfterUpload.set(true);
                return response(chain.request(), 200, "{\"text\":\"complete raw\"}");
            }).build();
            check("complete raw".equals(DirectOpenAi.process(cancelling, "test-only-key", new byte[]{1},
                    "minimal", true, cancelledAfterUpload, active -> {})) && requests.size() == 1,
                    "Cancellation after upload prevents a polish request");
            boolean invalidTranscriptRejected = false;
            try {
                DirectOpenAi.parseTranscription("{\"text\":123}");
            } catch (DirectOpenAi.DirectError expected) {
                invalidTranscriptRejected = true;
            }
            check(invalidTranscriptRejected, "Non-text transcript is rejected");
            check("low".equals(DirectOpenAi.reasoningEffort("original"))
                    && "low".equals(DirectOpenAi.reasoningEffort("minimal"))
                    && "low".equals(DirectOpenAi.reasoningEffort("clean"))
                    && "medium".equals(DirectOpenAi.reasoningEffort("professional")),
                    "Five style reasoning tiers are preserved");
            result.putString("stream", "PASS: " + checks + " offline mocked direct-engine checks\n");
            status = Activity.RESULT_OK;
        } catch (Exception | AssertionError error) {
            result.putString("stream", "FAIL after " + checks + " checks: " + error.getClass().getSimpleName()
                    + (error instanceof AssertionError ? ": " + error.getMessage() : "") + "\n");
        }
        finish(status, result);
    }

    private static Response response(Request request, int code, String body) {
        return new Response.Builder().request(request).protocol(Protocol.HTTP_1_1).code(code).message("mock")
                .body(ResponseBody.create(body, okhttp3.MediaType.get("application/json"))).build();
    }

    private void check(boolean condition, String label) {
        checks++;
        if (!condition) {
            throw new AssertionError(label);
        }
    }
}
