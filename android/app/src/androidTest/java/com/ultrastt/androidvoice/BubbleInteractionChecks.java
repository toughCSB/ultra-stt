package com.ultrastt.androidvoice;

import android.app.Activity;
import android.app.Instrumentation;
import android.app.UiAutomation;
import android.graphics.Rect;
import android.os.Bundle;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.MotionEvent;
import android.view.accessibility.AccessibilityWindowInfo;

public final class BubbleInteractionChecks extends Instrumentation {
    private UiAutomation automation;
    private int checks;
    private Bundle options;

    @Override
    public void onCreate(Bundle arguments) {
        super.onCreate(arguments);
        options = arguments == null ? new Bundle() : arguments;
        start();
    }

    @Override
    public void onStart() {
        Bundle result = new Bundle();
        Preferences preferences = new Preferences(getTargetContext());
        float originalX = preferences.bubbleX();
        float originalY = preferences.bubbleY();
        String originalMode = preferences.mode();
        int status = Activity.RESULT_CANCELED;
        try {
            runOnMainSync(this::checkViewEvents);
            if ("view-only".equals(options.getString("scenario", "view-only"))) {
                result.putString("stream", "PASS: " + checks + " actual BubbleView gesture checks; no microphone or network\n");
                status = Activity.RESULT_OK;
                return;
            }
            automation = getUiAutomation(UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES);
            android.accessibilityservice.AccessibilityServiceInfo info = automation.getServiceInfo();
            info.flags |= android.accessibilityservice.AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS;
            automation.setServiceInfo(info);
            Rect before = awaitBubble();
            float density = getTargetContext().getResources().getDisplayMetrics().density;
            int screenHeight = getTargetContext().getResources().getDisplayMetrics().heightPixels;
            int screenWidth = getTargetContext().getResources().getDisplayMetrics().widthPixels;
            int bottom = screenHeight;
            for (AccessibilityWindowInfo window : automation.getWindows()) {
                if (window.getType() == AccessibilityWindowInfo.TYPE_INPUT_METHOD) {
                    Rect bounds = new Rect();
                    window.getBoundsInScreen(bounds);
                    bottom = Math.min(bottom, bounds.top);
                }
            }
            float destinationX = screenWidth * 0.30f;
            float destinationY = Math.min(before.centerY() + 140 * density, bottom - 48 * density);
            require(destinationY > before.centerY() + 40 * density,
                    "Open a top-positioned search field to test downward dragging");
            drag(before.centerX(), before.centerY(), destinationX, destinationY, false);
            Rect moved = awaitBubble();
            require(Math.abs(moved.centerY() - destinationY) < 10 * density,
                    "Top search field must allow dragging down to requested position: " + before + " -> " + moved);
            require(Math.abs(moved.centerX() - destinationX) < 10 * density, "Horizontal drag must follow finger");
            SystemClock.sleep(500);
            require(moved.equals(awaitBubble()), "Editor refresh must preserve dragged position");
            drag(moved.centerX(), moved.centerY(), moved.centerX() + 50 * density, moved.centerY(), true);
            require(moved.equals(awaitBubble()), "Cancelled drag must restore its starting position");
            long time = SystemClock.uptimeMillis();
            inject(time, MotionEvent.ACTION_DOWN, moved.centerX(), moved.centerY());
            SystemClock.sleep(android.view.ViewConfiguration.getLongPressTimeout() + 180);
            require(overlay(true) != null, "Stationary long press must still open radial mode menu");
            inject(time, MotionEvent.ACTION_CANCEL, moved.centerX(), moved.centerY());
            require(moved.equals(awaitBubble()), "Cancelling mode menu must not move microphone");
            require(originalMode.equals(preferences.mode()), "Drag and cancelled mode selection must preserve style");
            result.putString("stream", "PASS: " + checks + " bubble interaction checks; actual search-field drag "
                    + before + " -> " + moved + "; no microphone or network request; saved position/style restored\n");
            status = Activity.RESULT_OK;
        } catch (Exception | AssertionError error) {
            result.putString("stream", "FAIL: " + error.getClass().getSimpleName() + ": " + error.getMessage() + "\n");
        } finally {
            runOnMainSync(() -> {
                float restoreX = options.containsKey("restoreX") ? Float.parseFloat(options.getString("restoreX")) : originalX;
                float restoreY = options.containsKey("restoreY") ? Float.parseFloat(options.getString("restoreY")) : originalY;
                if (Float.isFinite(restoreX) && Float.isFinite(restoreY)) {
                    preferences.bubblePosition(restoreX, restoreY);
                } else {
                    preferences.resetBubblePosition();
                }
                preferences.mode(originalMode);
                VoiceAccessibilityService service = VoiceAccessibilityService.active();
                if (service != null) service.settingsChanged();
            });
            getTargetContext().getSharedPreferences("voice-settings", android.content.Context.MODE_PRIVATE).edit().commit();
            finish(status, result);
        }
    }

    private void checkViewEvents() {
        int[] calls = new int[4];
        float[] delta = new float[2];
        BubbleView view = new BubbleView(getTargetContext(), new BubbleView.Gestures() {
            public void tap() { calls[0]++; }
            public void hold() { calls[1]++; }
            public void drag(float screenX, float screenY) {}
            public void release(float screenX, float screenY) { calls[2]++; }
            public void cancel() {}
            public void move(float deltaX, float deltaY) { delta[0] = deltaX; delta[1] = deltaY; }
            public void finishMove(boolean cancelled) { calls[3]++; }
        });
        touch(view, MotionEvent.ACTION_DOWN, 20, 20);
        touch(view, MotionEvent.ACTION_UP, 180, 140);
        require(calls[0] == 0 && calls[3] == 1 && delta[0] == 160 && delta[1] == 120,
                "Fast release without MOVE must move to final pointer, never record");
        touch(view, MotionEvent.ACTION_DOWN, 20, 20);
        touch(view, MotionEvent.ACTION_UP, 20, 20);
        require(calls[0] == 1, "Stationary tap still starts recording callback");
        touch(view, MotionEvent.ACTION_DOWN, 20, 20);
        require(view.performLongClick(), "Stationary hold must remain available");
        touch(view, MotionEvent.ACTION_UP, 180, 140);
        require(calls[1] == 1 && calls[2] == 1 && calls[3] == 1 && calls[0] == 1,
                "Release after long press selects style instead of moving or recording");
        touch(view, MotionEvent.ACTION_DOWN, 20, 20);
        touch(view, MotionEvent.ACTION_POINTER_DOWN, 20, 20);
        touch(view, MotionEvent.ACTION_UP, 180, 140);
        require(calls[0] == 1 && calls[3] == 1, "Second finger cancels without a late tap or movement");
        view.cancelGesture();
    }

    private void touch(BubbleView view, int action, float positionX, float positionY) {
        long now = SystemClock.uptimeMillis();
        MotionEvent event = MotionEvent.obtain(now, now, action, positionX, positionY, 0);
        try { view.onTouchEvent(event); } finally { event.recycle(); }
    }

    private Rect overlay(boolean menu) {
        float density = getTargetContext().getResources().getDisplayMetrics().density;
        for (AccessibilityWindowInfo window : automation.getWindows()) {
            if (window.getType() != AccessibilityWindowInfo.TYPE_ACCESSIBILITY_OVERLAY) continue;
            android.view.accessibility.AccessibilityNodeInfo root = window.getRoot();
            if (root == null || !"com.ultrastt.androidvoice".contentEquals(root.getPackageName())) continue;
            Rect bounds = new Rect();
            window.getBoundsInScreen(bounds);
            if (menu ? bounds.width() > 100 * density : Math.abs(bounds.width() - 64 * density) < 4) return bounds;
        }
        return null;
    }

    private Rect awaitBubble() {
        long deadline = SystemClock.uptimeMillis() + 5000;
        Rect previous = null;
        int stable = 0;
        do {
            Rect current = overlay(false);
            stable = current != null && current.equals(previous) ? stable + 1 : 0;
            if (stable >= 3) return current;
            previous = current;
            SystemClock.sleep(100);
        } while (SystemClock.uptimeMillis() < deadline);
        throw new AssertionError("Microphone overlay missing; unlock phone and focus search field");
    }

    private void drag(float fromX, float fromY, float toX, float toY, boolean cancel) {
        long time = SystemClock.uptimeMillis();
        inject(time, MotionEvent.ACTION_DOWN, fromX, fromY);
        for (int step = 1; step <= 6; step++) {
            inject(time, MotionEvent.ACTION_MOVE, fromX + (toX - fromX) * step / 6,
                    fromY + (toY - fromY) * step / 6);
            SystemClock.sleep(25);
        }
        inject(time, cancel ? MotionEvent.ACTION_CANCEL : MotionEvent.ACTION_UP, toX, toY);
    }

    private void inject(long time, int action, float positionX, float positionY) {
        MotionEvent event = MotionEvent.obtain(time, SystemClock.uptimeMillis(), action, positionX, positionY, 0);
        event.setSource(InputDevice.SOURCE_TOUCHSCREEN);
        try { require(automation.injectInputEvent(event, true), "Touch injection failed"); }
        finally { event.recycle(); }
    }

    private void require(boolean condition, String message) {
        checks++;
        if (!condition) throw new AssertionError(message);
    }
}
