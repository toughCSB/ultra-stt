package com.ultrastt.androidvoice;

import android.content.Context;
import android.content.SharedPreferences;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import android.util.Base64;

import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import java.security.KeyStore;

import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

final class Preferences {
    private static final String KEY_ALIAS = "ultra-stt-remote-token-v1";
    private static final String OPENAI_KEY_ALIAS = "ultra-stt-openai-key-v1";
    private final SharedPreferences store;
    private final Context context;

    Preferences(Context context) {
        this.context = context.getApplicationContext();
        store = context.getSharedPreferences("voice-settings", Context.MODE_PRIVATE);
    }

    boolean floatingEnabled() {
        return store.getBoolean("floating-enabled", true);
    }

    float bubbleX() {
        return store.getFloat("bubble-x", Float.NaN);
    }

    float bubbleY() {
        return store.getFloat("bubble-y", Float.NaN);
    }

    void bubblePosition(float x, float y) {
        store.edit().putFloat("bubble-x", x).putFloat("bubble-y", y).apply();
    }

    void resetBubblePosition() {
        store.edit().remove("bubble-x").remove("bubble-y").apply();
    }

    void floatingEnabled(boolean enabled) {
        store.edit().putBoolean("floating-enabled", enabled).apply();
    }

    boolean importPairing() throws GeneralSecurityException {
        java.io.File pairing = new java.io.File(context.getFilesDir(), "pairing.json");
        if (!pairing.exists()) {
            return false;
        }
        try {
            if (pairing.length() > 4096) {
                throw new GeneralSecurityException("연결 정보 파일이 올바르지 않습니다.");
            }
            byte[] bytes;
            try (java.io.FileInputStream input = new java.io.FileInputStream(pairing)) {
                java.io.ByteArrayOutputStream buffer = new java.io.ByteArrayOutputStream();
                byte[] chunk = new byte[512];
                int count;
                while ((count = input.read(chunk)) != -1) {
                    if (buffer.size() + count > 4096) {
                        throw new GeneralSecurityException("연결 정보 파일이 올바르지 않습니다.");
                    }
                    buffer.write(chunk, 0, count);
                }
                bytes = buffer.toByteArray();
            }
            org.json.JSONObject configuration = new org.json.JSONObject(new String(bytes, StandardCharsets.UTF_8));
            long expiry = configuration.getLong("expiresAt");
            long now = System.currentTimeMillis();
            if (expiry < now || expiry > now + 15 * 60 * 1000) {
                throw new GeneralSecurityException("연결 정보가 만료됐습니다. PC에서 다시 연결해 주세요.");
            }
            save(configuration.getString("endpoint"), configuration.getString("token"),
                    configuration.optString("serverAddress", ""));
            return true;
        } catch (java.io.IOException | org.json.JSONException | IllegalArgumentException error) {
            throw new GeneralSecurityException("PC 연결 정보를 읽지 못했습니다. 다시 연결해 주세요.");
        } finally {
            if (!pairing.delete()) {
                try (java.io.FileOutputStream erase = new java.io.FileOutputStream(pairing)) {
                    erase.write(new byte[0]);
                } catch (java.io.IOException ignored) {}
            }
        }
    }

    String endpoint() {
        return store.getString("endpoint", "https://sb.tail337f88.ts.net:10443/v1/audio/transcriptions");
    }

    String mode() {
        return Modes.IDS[Modes.index(store.getString("mode", "minimal"))];
    }

    String engine() {
        return Engine.normalize(store.getString("engine", Engine.LEGACY));
    }

    void engine(String value) {
        store.edit().putString("engine", Engine.normalize(value)).apply();
    }

    boolean hasOpenAiKey() {
        return store.contains("openai-key-encrypted") && store.contains("openai-key-nonce");
    }

    String openAiKey() throws GeneralSecurityException {
        String encrypted = store.getString("openai-key-encrypted", null);
        String nonce = store.getString("openai-key-nonce", null);
        if (encrypted == null || nonce == null) {
            return "";
        }
        try {
            Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
            cipher.init(Cipher.DECRYPT_MODE, key(OPENAI_KEY_ALIAS),
                    new GCMParameterSpec(128, Base64.decode(nonce, Base64.NO_WRAP)));
            return new String(cipher.doFinal(Base64.decode(encrypted, Base64.NO_WRAP)), StandardCharsets.UTF_8);
        } catch (IllegalArgumentException error) {
            throw new GeneralSecurityException("저장된 OpenAI API 키를 다시 입력해 주세요.");
        }
    }

    void saveOpenAiKey(String value) throws GeneralSecurityException {
        String credential = value.trim();
        if (credential.isEmpty()) {
            throw new IllegalArgumentException("OpenAI API 키를 입력해 주세요.");
        }
        for (int index = 0; index < credential.length(); index++) {
            char character = credential.charAt(index);
            if (character <= 0x20 || character >= 0x7f) {
                throw new IllegalArgumentException("OpenAI API 키에 사용할 수 없는 문자가 있습니다.");
            }
        }
        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.ENCRYPT_MODE, key(OPENAI_KEY_ALIAS));
        if (!store.edit().putString("openai-key-encrypted", Base64.encodeToString(
                cipher.doFinal(credential.getBytes(StandardCharsets.UTF_8)), Base64.NO_WRAP))
                .putString("openai-key-nonce", Base64.encodeToString(cipher.getIV(), Base64.NO_WRAP)).commit()) {
            throw new GeneralSecurityException("OpenAI API 키를 저장하지 못했습니다.");
        }
    }

    void clearOpenAiKey() throws GeneralSecurityException {
        if (!store.edit().remove("openai-key-encrypted").remove("openai-key-nonce").commit()) {
            throw new GeneralSecurityException("OpenAI API 키를 삭제하지 못했습니다.");
        }
    }

    String serverAddress() {
        return PairedAddress.forEndpoint(endpoint(), store.getString("paired-host", ""),
                store.getString("paired-address", ""));
    }

    void observe(SharedPreferences.OnSharedPreferenceChangeListener listener) {
        store.registerOnSharedPreferenceChangeListener(listener);
    }

    void unobserve(SharedPreferences.OnSharedPreferenceChangeListener listener) {
        store.unregisterOnSharedPreferenceChangeListener(listener);
    }

    void mode(String value) {
        store.edit().putString("mode", Modes.IDS[Modes.index(value)]).apply();
    }

    boolean polishEnabled() {
        return store.getBoolean("polish-enabled", true);
    }

    void polishEnabled(boolean enabled) {
        store.edit().putBoolean("polish-enabled", enabled).apply();
    }

    boolean hasToken() {
        return store.contains("token-encrypted");
    }

    String token() throws GeneralSecurityException {
        String encrypted = store.getString("token-encrypted", null);
        String nonce = store.getString("token-nonce", null);
        if (encrypted == null || nonce == null) {
            return "";
        }
        try {
            Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
            cipher.init(Cipher.DECRYPT_MODE, key(KEY_ALIAS), new GCMParameterSpec(128, Base64.decode(nonce, Base64.NO_WRAP)));
            return new String(cipher.doFinal(Base64.decode(encrypted, Base64.NO_WRAP)), StandardCharsets.UTF_8);
        } catch (IllegalArgumentException error) {
            throw new GeneralSecurityException("저장된 인증 정보를 다시 입력해 주세요.");
        }
    }

    void save(String endpoint, String token) throws GeneralSecurityException {
        save(endpoint, token, PairedAddress.forEndpoint(endpoint, store.getString("paired-host", ""),
                store.getString("paired-address", "")));
    }

    private void save(String endpoint, String token, String address) throws GeneralSecurityException {
        String normalized = Endpoint.normalize(endpoint);
        SharedPreferences.Editor editor = store.edit().putString("endpoint", normalized);
        if (address.isEmpty()) {
            editor.remove("paired-host").remove("paired-address");
        } else {
            String host = PairedAddress.host(normalized);
            if (PairedAddress.forEndpoint(normalized, host, address).isEmpty()) {
                throw new IllegalArgumentException("Tailscale 서버 주소를 확인해 주세요.");
            }
            editor.putString("paired-host", host).putString("paired-address", address);
        }
        if (!token.isEmpty()) {
            if (!Endpoint.validToken(token)) {
                throw new IllegalArgumentException("서버의 96자리 인증 토큰을 입력해 주세요.");
            }
            Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
            cipher.init(Cipher.ENCRYPT_MODE, key(KEY_ALIAS));
            editor.putString("token-encrypted", Base64.encodeToString(
                    cipher.doFinal(token.getBytes(StandardCharsets.UTF_8)), Base64.NO_WRAP));
            editor.putString("token-nonce", Base64.encodeToString(cipher.getIV(), Base64.NO_WRAP));
        } else if (!Endpoint.validToken(token())) {
            throw new IllegalArgumentException("최초 설정에는 서버 인증 토큰이 필요합니다.");
        }
        if (!editor.commit()) {
            throw new GeneralSecurityException("설정을 저장하지 못했습니다.");
        }
    }

    private SecretKey key(String alias) throws GeneralSecurityException {
        KeyStore keys = KeyStore.getInstance("AndroidKeyStore");
        try {
            keys.load(null);
        } catch (java.io.IOException error) {
            throw new GeneralSecurityException("안전한 인증 저장소를 열지 못했습니다.");
        }
        if (keys.containsAlias(alias)) {
            return (SecretKey) keys.getKey(alias, null);
        }
        KeyGenerator generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore");
        generator.init(new KeyGenParameterSpec.Builder(alias,
                KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256).build());
        return generator.generateKey();
    }
}
