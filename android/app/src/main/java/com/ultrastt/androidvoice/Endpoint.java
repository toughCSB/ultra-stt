package com.ultrastt.androidvoice;

import java.net.URI;
import java.net.URISyntaxException;

final class Endpoint {
    static String normalize(String value) {
        try {
            URI endpoint = new URI(value.trim());
            if (!"https".equalsIgnoreCase(endpoint.getScheme()) || endpoint.getHost() == null
                    || endpoint.getUserInfo() != null || endpoint.getRawQuery() != null
                    || endpoint.getRawFragment() != null || endpoint.getPort() > 65535
                    || endpoint.getPort() == 0) {
                throw new IllegalArgumentException("HTTPS 서버 주소를 입력해 주세요. 사용자 정보·쿼리·앵커는 허용되지 않습니다.");
            }
            String path = endpoint.getPath();
            if (path == null || path.isEmpty() || "/".equals(path)) {
                path = "/v1/audio/transcriptions";
            } else if ("/v1".equals(path) || "/v1/".equals(path)) {
                path = "/v1/audio/transcriptions";
            } else if (!"/v1/audio/transcriptions".equals(path)) {
                throw new IllegalArgumentException("서버 기본 주소 또는 /v1/audio/transcriptions 주소를 입력해 주세요.");
            }
            return new URI("https", null, endpoint.getHost(), endpoint.getPort(), path, null, null).toASCIIString();
        } catch (URISyntaxException error) {
            throw new IllegalArgumentException("올바른 HTTPS 서버 주소를 입력해 주세요.");
        }
    }

    static boolean validToken(String value) {
        return value != null && value.matches("[0-9a-fA-F]{96}");
    }

    private Endpoint() {}
}
