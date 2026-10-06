package com.ultrastt.androidvoice;

final class BubblePlacement {
    static final int TOUCH_SIZE = 64;
    static final int CIRCLE_SIZE = 56;
    static final float IDLE_ALPHA = 0.74f;

    static float defaultTop(float fieldTop, float fieldBottom, float keyboardTop, boolean keyboardVisible) {
        float aboveField = fieldTop - TOUCH_SIZE - 48;
        return keyboardVisible ? Math.min(aboveField, keyboardTop - TOUCH_SIZE - 112) : aboveField;
    }

    static float clamp(float value, float minimum, float maximum) {
        return Math.max(minimum, Math.min(value, Math.max(minimum, maximum)));
    }

    static float highestAllowedBottom(float fieldTop, float availableBottom, float keyboardTop) {
        return Math.min(fieldTop - 48, Math.min(availableBottom, keyboardTop - 8));
    }

    static float fraction(float position, float minimum, float maximum) {
        return maximum <= minimum ? 0 : clamp((position - minimum) / (maximum - minimum), 0, 1);
    }

    static float restore(float fraction, float minimum, float maximum) {
        return minimum + clamp(fraction, 0, 1) * Math.max(0, maximum - minimum);
    }

    private BubblePlacement() {}
}
