package com.ultrastt.androidvoice;

final class BubbleGesture {
    enum State { IDLE, PRESSED, MOVING, SELECTING }

    private State state = State.IDLE;
    private float downX;
    private float downY;

    State state() {
        return state;
    }

    void down(float screenX, float screenY) {
        downX = screenX;
        downY = screenY;
        state = State.PRESSED;
    }

    State move(float screenX, float screenY, float slop) {
        if (state == State.PRESSED && Math.hypot(screenX - downX, screenY - downY) > slop) {
            state = State.MOVING;
        }
        return state;
    }

    boolean hold() {
        if (state != State.PRESSED) {
            return false;
        }
        state = State.SELECTING;
        return true;
    }

    float deltaX(float screenX) {
        return screenX - downX;
    }

    float deltaY(float screenY) {
        return screenY - downY;
    }

    State finish() {
        State previous = state;
        state = State.IDLE;
        return previous;
    }
}
