package com.ultrastt.androidvoice;

import android.app.Activity;
import android.os.Bundle;
import android.view.Gravity;
import android.view.WindowManager;

public final class RecordingActivity extends Activity {
    private VoiceAccessibilityService service;
    private BubbleView bubble;
    private long session;
    private boolean completed;
    private boolean started;

    @Override
    public void onCreate(Bundle state) {
        super.onCreate(state);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE
                | WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL | WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN);
        getWindow().clearFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND);
        setFinishOnTouchOutside(false);
        session = getIntent().getLongExtra("session", -1);
        service = VoiceAccessibilityService.active();
        if (service == null || !service.attachRecorder(this, session)) {
            completed = true;
            finish();
            return;
        }
        bubble = new BubbleView(this, service.gestures());
        setContentView(bubble);
        position(getIntent().getIntExtra("x", 0), getIntent().getIntExtra("y", 0));
    }

    void position(int x, int y) {
        WindowManager.LayoutParams params = getWindow().getAttributes();
        params.gravity = Gravity.TOP | Gravity.LEFT;
        params.width = Ui.dp(this, 56);
        params.height = Ui.dp(this, 56);
        params.x = x;
        params.y = y;
        if (android.os.Build.VERSION.SDK_INT >= 30) {
            params.setFitInsetsTypes(0);
            params.layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS;
        }
        getWindow().setAttributes(params);
    }

    @Override
    protected void onResume() {
        super.onResume();
        if (!completed && !started && service != null) {
            started = true;
            bubble.post(() -> {
                if (!completed) {
                    service.beginCapture(session);
                }
            });
        }
    }

    void update(boolean recording, boolean processing, String mode) {
        if (bubble != null) {
            bubble.update(recording, processing, mode);
        }
    }

    void progress(int seconds) {
        if (bubble != null) {
            bubble.progress(seconds);
        }
    }

    void complete() {
        completed = true;
        finish();
        overridePendingTransition(0, 0);
    }

    @Override
    protected void onPause() {
        if (!completed && service != null) {
            service.recorderStopped(this);
        }
        super.onPause();
    }

    @Override
    protected void onDestroy() {
        if (!completed && service != null) {
            service.recorderStopped(this);
        }
        super.onDestroy();
    }
}
