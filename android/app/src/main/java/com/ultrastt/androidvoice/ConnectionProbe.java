package com.ultrastt.androidvoice;

import org.json.JSONObject;

import okhttp3.Request;
import okhttp3.RequestBody;
import okhttp3.Response;

final class ConnectionProbe {
    static final class Result {
        final boolean verified;
        final String message;

        Result(boolean verified, String message) {
            this.verified = verified;
            this.message = message;
        }
    }

    static Result check(String endpoint, String token, String address) {
        try {
            if (!Endpoint.validToken(token)) {
                return failure("PC 연결 정보를 다시 받아 주세요.");
            }
            Request request = new Request.Builder().url(Endpoint.normalize(endpoint))
                    .header("Authorization", "Bearer " + token)
                    .post(RequestBody.create(new byte[0], null)).build();
            try (Response response = HttpTransport.client(endpoint, address, true).newCall(request).execute()) {
                int status = response.code();
                if (status != 413) {
                    return failure(status == 401 || status == 403 ? "PC 인증이 맞지 않습니다. 연결 정보를 다시 받아 주세요."
                            : "PC 서버 응답을 확인해 주세요. (HTTP " + status + ")");
                }
                JSONObject body = new JSONObject(HttpTransport.read(response.body(), 4096));
                return "audio too large".equals(body.optString("error"))
                        ? new Result(true, "PC 서버 연결·인증 확인됨")
                        : failure("PC 서버의 연결 확인 응답이 올바르지 않습니다.");
            }
        } catch (java.net.UnknownHostException error) {
            return failure("PC 주소를 찾지 못했습니다. 앱 전용 연결 정보를 다시 받아 주세요.");
        } catch (javax.net.ssl.SSLException error) {
            return failure("HTTPS 인증서를 확인하지 못했습니다. 서버 주소와 기기 시간을 확인해 주세요.");
        } catch (java.io.InterruptedIOException error) {
            return failure("PC 서버 응답 시간이 초과됐습니다. PC와 Tailscale 연결을 확인해 주세요.");
        } catch (Exception error) {
            return failure("PC 서버에 연결하지 못했습니다. PC와 Tailscale 연결을 확인해 주세요.");
        }
    }

    private static Result failure(String message) {
        return new Result(false, message);
    }

    private ConnectionProbe() {}
}
