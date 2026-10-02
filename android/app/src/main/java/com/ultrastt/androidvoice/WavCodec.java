package com.ultrastt.androidvoice;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;

final class WavCodec {
    static final int SAMPLE_RATE = 16000;
    static final int MAX_SECONDS = 120;
    static final int MAX_PCM_BYTES = SAMPLE_RATE * 2 * MAX_SECONDS;

    static byte[] encode(byte[] pcm) {
        if (pcm.length == 0 || pcm.length % 2 != 0 || pcm.length > MAX_PCM_BYTES) {
            throw new IllegalArgumentException("유효한 16 kHz PCM16 녹음이 아닙니다.");
        }
        ByteBuffer wav = ByteBuffer.allocate(44 + pcm.length).order(ByteOrder.LITTLE_ENDIAN);
        wav.put("RIFF".getBytes(StandardCharsets.US_ASCII)).putInt(36 + pcm.length);
        wav.put("WAVEfmt ".getBytes(StandardCharsets.US_ASCII)).putInt(16);
        wav.putShort((short) 1).putShort((short) 1).putInt(SAMPLE_RATE);
        wav.putInt(SAMPLE_RATE * 2).putShort((short) 2).putShort((short) 16);
        wav.put("data".getBytes(StandardCharsets.US_ASCII)).putInt(pcm.length).put(pcm);
        return wav.array();
    }

    private WavCodec() {}
}
