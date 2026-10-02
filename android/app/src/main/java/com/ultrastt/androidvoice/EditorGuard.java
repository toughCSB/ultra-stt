package com.ultrastt.androidvoice;

final class EditorGuard {
    private long generation;
    private boolean active;

    long begin() {
        active = true;
        return ++generation;
    }

    void invalidate() {
        active = false;
        generation++;
    }

    boolean accepts(long expected) {
        return active && generation == expected;
    }

    boolean isActive() {
        return active;
    }

    EditorGuard() {}
}
