package com.ultrastt.androidvoice;

import android.app.Activity;
import android.app.Instrumentation;
import android.os.Bundle;

import java.io.IOException;

import okhttp3.Call;
import okhttp3.OkHttpClient;
import okhttp3.Request;
import okhttp3.RequestBody;
import okhttp3.Response;
import okhttp3.ResponseBody;

public final class NetworkChecks extends Instrumentation {
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
            OkHttpClient internet = new OkHttpClient.Builder().callTimeout(10, java.util.concurrent.TimeUnit.SECONDS).build();
            try (Response response = internet.newCall(new Request.Builder()
                    .url("https://www.google.com/generate_204").build()).execute()) {
                check(response.code() == 204, "Ordinary DNS and public HTTPS work");
            }
            Preferences preferences = new Preferences(getTargetContext());
            String endpoint = preferences.endpoint();
            String address = preferences.serverAddress();
            String token = preferences.token();
            check(!address.isEmpty(), "Device must be privately paired");
            OkHttpClient client = HttpTransport.client(endpoint, address, true);
            check(client.dns().lookup(PairedAddress.host(endpoint)).get(0).getHostAddress().equals(address),
                    "Only the paired host uses its direct address");
            check(client.hostnameVerifier() == new OkHttpClient().hostnameVerifier(), "Default TLS hostname verification preserved");
            check(!client.followRedirects() && !client.followSslRedirects(), "Redirects cannot forward credentials");
            check(ConnectionProbe.check(endpoint, token, address).verified, "Real device HTTPS and Bearer accepted");
            String wrong = (token.charAt(0) == 'a' ? "b" : "a") + token.substring(1);
            ConnectionProbe.Result denied = ConnectionProbe.check(endpoint, wrong, address);
            check(!denied.verified && denied.message.contains("인증"), "Wrong Bearer rejected by real server");
            String wrongEndpoint = endpoint.replace(PairedAddress.host(endpoint), "wrong-certificate.ts.net");
            Request invalid = new Request.Builder().url(wrongEndpoint).post(RequestBody.create(new byte[0], null)).build();
            boolean tlsRejected = false;
            try (Response response = HttpTransport.client(wrongEndpoint, address, true).newCall(invalid).execute()) {
                tlsRejected = false;
            } catch (IOException expected) {
                tlsRejected = true;
            }
            check(tlsRejected, "Direct address does not accept another HTTPS hostname");
            Call cancelled = client.newCall(new Request.Builder().url(endpoint).post(RequestBody.create(new byte[0], null)).build());
            cancelled.cancel();
            boolean cancellationRejected = false;
            try (Response response = cancelled.execute()) {
                cancellationRejected = false;
            } catch (IOException expected) {
                cancellationRejected = true;
            }
            check(cancellationRejected, "Cancelled call cannot deliver a response");
            try (ResponseBody body = ResponseBody.create("bounded", null)) {
                check(HttpTransport.read(body, 7).equals("bounded"), "Bounded response accepted");
            }
            boolean oversizedRejected = false;
            try (ResponseBody body = ResponseBody.create("oversized", null)) {
                HttpTransport.read(body, 3);
            } catch (IOException expected) {
                oversizedRejected = true;
            }
            check(oversizedRejected, "Oversized response rejected");
            result.putString("stream", "PASS: " + checks + " device transport checks"
                    + "; no audio or AI request\n");
            status = Activity.RESULT_OK;
        } catch (Exception | AssertionError error) {
            result.putString("stream", "FAIL after " + checks + " checks: " + error.getClass().getSimpleName()
                    + (error instanceof AssertionError ? ": " + error.getMessage() : "") + "\n");
        } finally {
            try (java.io.FileOutputStream output = getTargetContext().openFileOutput("network-checks.txt", 0)) {
                output.write(result.getString("stream", "No result").getBytes(java.nio.charset.StandardCharsets.UTF_8));
            } catch (IOException error) {
                result.putString("stream", "FAIL: QA report could not be saved\n");
                status = Activity.RESULT_CANCELED;
            }
        }
        finish(status, result);
    }

    private void check(boolean condition, String label) {
        checks++;
        if (!condition) {
            throw new AssertionError(label);
        }
    }
}
