package com.ultrastt.androidvoice;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;

public final class VoiceTests {
    private static int checks;

    public static void main(String[] args) {
        byte[] pcm = {0, 0, -1, 127, 0, -128};
        byte[] wav = WavCodec.encode(pcm);
        ByteBuffer header = ByteBuffer.wrap(wav).order(ByteOrder.LITTLE_ENDIAN);
        check(wav.length == pcm.length + 44, "WAV length");
        check(new String(wav, 0, 4, StandardCharsets.US_ASCII).equals("RIFF"), "RIFF magic");
        check(header.getInt(4) == wav.length - 8, "RIFF size");
        check(header.getShort(20) == 1 && header.getShort(22) == 1, "mono PCM format");
        check(header.getInt(24) == 16000 && header.getShort(34) == 16, "16 kHz PCM16");
        check(header.getInt(40) == pcm.length, "PCM byte count");
        check(Arrays.equals(pcm, Arrays.copyOfRange(wav, 44, wav.length)), "PCM preserved");
        reject(() -> WavCodec.encode(new byte[0]), "empty audio");
        reject(() -> WavCodec.encode(new byte[3]), "odd PCM length");
        reject(() -> WavCodec.encode(new byte[WavCodec.MAX_PCM_BYTES + 2]), "oversized audio");
        check(WavCodec.encode(new byte[WavCodec.MAX_PCM_BYTES]).length == WavCodec.MAX_PCM_BYTES + 44, "120 second limit");

        String target = "https://server.example:10443/v1/audio/transcriptions";
        check(Endpoint.normalize(" https://server.example:10443/ ").equals(target), "base URL normalization");
        check(Endpoint.normalize("https://server.example:10443/v1").equals(target), "v1 normalization");
        check(Endpoint.normalize(target).equals(target), "full endpoint");
        reject(() -> Endpoint.normalize("http://server.example"), "cleartext rejected");
        reject(() -> Endpoint.normalize("https://secret@server.example"), "URL credential rejected");
        reject(() -> Endpoint.normalize(target + "?token=secret"), "query rejected");
        reject(() -> Endpoint.normalize(target + "#secret"), "fragment rejected");
        reject(() -> Endpoint.normalize("https://server.example/ws"), "wrong route rejected");
        reject(() -> Endpoint.normalize("https://server.example:70000"), "invalid port rejected");
        reject(() -> Endpoint.normalize("not a server"), "invalid URI rejected");
        check(Endpoint.validToken("a".repeat(96)), "valid token");
        check(!Endpoint.validToken("Bearer secret") && !Endpoint.validToken("a".repeat(95)), "invalid token");
        check(!Endpoint.validToken("a".repeat(96) + "\r\nInjected: true"), "header injection rejected");
        String paired = "https://pc.tail1234.ts.net:10443/v1/audio/transcriptions";
        check(PairedAddress.host(paired).equals("pc.tail1234.ts.net"), "HTTPS hostname remains intact");
        check(Arrays.equals(PairedAddress.bytes("100.64.0.1"), new byte[]{100, 64, 0, 1}), "tailnet lower bound");
        check(Arrays.equals(PairedAddress.bytes("100.127.255.254"), new byte[]{100, 127, -1, -2}), "tailnet upper bound");
        reject(() -> PairedAddress.bytes("100.63.1.1"), "below tailnet rejected");
        reject(() -> PairedAddress.bytes("100.128.1.1"), "above tailnet rejected");
        reject(() -> PairedAddress.bytes("192.168.1.1"), "LAN pin rejected");
        reject(() -> PairedAddress.bytes("100.70.50.999"), "invalid octet rejected");
        reject(() -> PairedAddress.bytes("100.070.50.88"), "ambiguous octet rejected");
        reject(() -> PairedAddress.bytes("server.example"), "pin cannot trigger DNS lookup");
        reject(() -> PairedAddress.bytes("100.70.50.88\n"), "pin whitespace rejected");
        reject(() -> PairedAddress.bytes(null), "null pin rejected");
        check(PairedAddress.forEndpoint(paired, "pc.tail1234.ts.net", "100.70.50.88").equals("100.70.50.88"), "paired route selected");
        check(PairedAddress.forEndpoint(paired, "other.tail1234.ts.net", "100.70.50.88").isEmpty(), "changed hostname drops pin");
        check(PairedAddress.forEndpoint(target, "server.example", "100.70.50.88").isEmpty(), "non-tailnet hostname cannot be pinned");
        check(PairedAddress.forEndpoint(paired, "pc.tail1234.ts.net", "").isEmpty(), "unpaired endpoint uses normal DNS");
        for (int index = 0; index < Modes.IDS.length; index++) {
            check(Modes.index(Modes.IDS[index]) == index, "mode " + Modes.IDS[index]);
        }
        check(Modes.IDS[Modes.index("unknown")].equals("minimal"), "safe fallback");

        EditorGuard guard = new EditorGuard();
        check(!guard.isActive(), "initially idle");
        long first = guard.begin();
        check(guard.accepts(first), "current recording accepted");
        guard.invalidate();
        check(!guard.accepts(first) && !guard.isActive(), "cancel drops pending result");
        long second = guard.begin();
        check(!guard.accepts(first) && guard.accepts(second), "new editor rejects old result");
        long third = guard.begin();
        check(!guard.accepts(second) && guard.accepts(third), "new capture rejects old result");
        guard.invalidate();
        check(!guard.accepts(third), "hidden keyboard rejects result");
        check(TextInsertion.replaceSelection("앞 뒤", 2, 2, "음성 ").equals("앞 음성 뒤"), "cursor insertion preserves text");
        check(TextInsertion.replaceSelection("앞 선택 뒤", 2, 4, "음성").equals("앞 음성 뒤"), "selected text replaced");
        check(TextInsertion.replaceSelection("앞 선택 뒤", 4, 2, "음성").equals("앞 음성 뒤"), "reversed selection replaced");
        check(TextInsertion.replaceSelection("", 0, 0, "한국어\n음성").equals("한국어\n음성"), "empty editor and multiline Korean");
        check(TextInsertion.replaceSelection("🙂뒤", 2, 2, "음성").equals("🙂음성뒤"), "emoji preserved");
        reject(() -> TextInsertion.replaceSelection("🙂뒤", 1, 1, "음성"), "split emoji rejected");
        reject(() -> TextInsertion.replaceSelection("입력", -1, 0, "음성"), "unknown selection rejected");
        reject(() -> TextInsertion.replaceSelection("입력", 0, 9, "음성"), "out of range selection rejected");
        String emptyEditor = TextInsertion.editorText("여기에 입력하세요", true);
        check(emptyEditor.isEmpty(), "placeholder is not editor content");
        check(TextInsertion.replaceSelection(emptyEditor,
                TextInsertion.editorSelection(emptyEditor, -1), TextInsertion.editorSelection(emptyEditor, -1),
                "음성").equals("음성"), "empty placeholder editor accepts safe cursor insertion");
        check(TextInsertion.editorText(null, false).isEmpty(), "null editor content is empty");
        check(TextInsertion.editorText("실제 입력", false).equals("실제 입력"), "actual input preserved");
        check(TextInsertion.editorSelection("실제 입력", 3) == 3, "known cursor preserved");
        reject(() -> TextInsertion.replaceSelection("실제 입력",
                TextInsertion.editorSelection("실제 입력", -1), 0, "음성"), "unknown nonempty cursor still rejected");
        for (int index = 0; index < 5; index++) {
            check(RadialGeometry.pick(RadialGeometry.x(index), RadialGeometry.y(index)) == index,
                    "radial drag selects " + Modes.IDS[index]);
            check(RadialGeometry.pick(RadialGeometry.x(index) * 1.2, RadialGeometry.y(index) * 1.2) == index,
                    "radial sector tolerant of distance " + index);
        }
        check(RadialGeometry.pick(0, 0) == -1, "hold and release at origin does not change mode");
        check(RadialGeometry.pick(-20, 0) == -1, "radial dead zone");
        check(RadialGeometry.pick(108, 0) == -1, "outside semicircle rejected");
        check(RadialGeometry.pick(-200, 0) == -1, "drag beyond menu cancels selection");
        check(RadialGeometry.pick(Double.NaN, 0) == -1, "invalid gesture rejected");
        System.out.println("PASS: " + checks + " Android floating voice checks");
    }

    private static void check(boolean condition, String label) {
        checks++;
        if (!condition) {
            throw new AssertionError(label);
        }
    }

    private static void reject(Runnable operation, String label) {
        boolean rejected = false;
        try {
            operation.run();
        } catch (IllegalArgumentException expected) {
            rejected = true;
        }
        check(rejected, label);
    }
}
