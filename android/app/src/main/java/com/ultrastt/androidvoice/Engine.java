package com.ultrastt.androidvoice;

final class Engine {
    static final String LEGACY = "legacy";
    static final String OPENAI = "openai";

    static String normalize(String value) {
        return OPENAI.equals(value) ? OPENAI : LEGACY;
    }

    private Engine() {}
}
