package com.ultrastt.androidvoice;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.net.InetAddress;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.concurrent.TimeUnit;

import okhttp3.Dns;
import okhttp3.OkHttpClient;
import okhttp3.ResponseBody;

final class HttpTransport {
    private static final OkHttpClient BASE = new OkHttpClient.Builder()
            .followRedirects(false).followSslRedirects(false).retryOnConnectionFailure(false)
            .connectTimeout(10, TimeUnit.SECONDS).writeTimeout(20, TimeUnit.SECONDS)
            .readTimeout(135, TimeUnit.SECONDS).callTimeout(150, TimeUnit.SECONDS).build();

    static OkHttpClient client(String endpoint, String address, boolean probe) {
        String host = PairedAddress.host(endpoint);
        OkHttpClient.Builder builder = BASE.newBuilder();
        if (!address.isEmpty()) {
            byte[] bytes = PairedAddress.bytes(address);
            if (!host.endsWith(".ts.net")) {
                throw new IllegalArgumentException("Tailscale 서버에만 전용 연결을 적용할 수 있습니다.");
            }
            builder.dns(name -> host.equals(name)
                    ? List.of(InetAddress.getByAddress(host, bytes)) : Dns.SYSTEM.lookup(name));
        }
        if (probe) {
            builder.connectTimeout(7, TimeUnit.SECONDS).writeTimeout(7, TimeUnit.SECONDS)
                    .readTimeout(7, TimeUnit.SECONDS).callTimeout(10, TimeUnit.SECONDS);
        }
        return builder.build();
    }

    static OkHttpClient openAiClient() {
        return BASE.newBuilder().dns(Dns.SYSTEM).build();
    }

    static String read(ResponseBody body, int limit) throws IOException {
        if (body == null) {
            throw new IOException("Missing response body");
        }
        try (InputStream input = body.byteStream(); ByteArrayOutputStream output = new ByteArrayOutputStream()) {
            byte[] buffer = new byte[4096];
            int count;
            while ((count = input.read(buffer)) != -1) {
                if (output.size() + count > limit) {
                    throw new IOException("Response limit exceeded");
                }
                output.write(buffer, 0, count);
            }
            return output.toString(StandardCharsets.UTF_8.name());
        }
    }

    private HttpTransport() {}
}
