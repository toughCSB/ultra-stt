package com.ultrastt.androidvoice;

import java.net.URI;
import java.util.Locale;

final class PairedAddress {
    static String host(String endpoint) {
        return URI.create(Endpoint.normalize(endpoint)).getHost().toLowerCase(Locale.ROOT);
    }

    static byte[] bytes(String address) {
        if (address == null || !address.matches("[0-9]{1,3}(\\.[0-9]{1,3}){3}")) {
            throw new IllegalArgumentException("올바른 Tailscale IPv4 주소가 필요합니다.");
        }
        String[] parts = address.split("\\.");
        byte[] bytes = new byte[4];
        for (int index = 0; index < parts.length; index++) {
            int value = Integer.parseInt(parts[index]);
            if (value > 255 || !parts[index].equals(Integer.toString(value))) {
                throw new IllegalArgumentException("올바른 Tailscale IPv4 주소가 필요합니다.");
            }
            bytes[index] = (byte) value;
        }
        if ((bytes[0] & 255) != 100 || (bytes[1] & 255) < 64 || (bytes[1] & 255) > 127) {
            throw new IllegalArgumentException("개인 Tailscale 주소만 연결할 수 있습니다.");
        }
        return bytes;
    }

    static String forEndpoint(String endpoint, String pairedHost, String address) {
        if (address == null || address.isEmpty()) {
            return "";
        }
        String host = host(endpoint);
        if (!host.endsWith(".ts.net") || !host.equals(pairedHost)) {
            return "";
        }
        bytes(address);
        return address;
    }

    private PairedAddress() {}
}
