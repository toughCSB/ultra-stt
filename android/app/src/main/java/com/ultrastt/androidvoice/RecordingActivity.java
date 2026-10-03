package com.ultrastt.androidvoice;

import android.app.Activity;
import android.os.Bundle;
import android.view.Gravity;
import android.view.WindowManager;

public final class RecordingActivity extends Activity {
    private VoiceAccessibilityService service;
    private RecordingPanelView panel;
    private final android.os.Handler main = new android.os.Handler(android.os.Looper.getMainLooper());
    private boolean closing;
    private long session;
    private boolean completed;
    private boolean started;
    private boolean resumed;

    boolean isRecordingForeground() {
        return resumed && !completed;
    }

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
        panel = new RecordingPanelView(this, ModeArtwork.load(this, R.drawable.mic_glyph),
                getDrawable(R.drawable.brand_wordmark),
                service::toggle, () -> service.cancelWork("음성 입력을 취소했습니다."), service::selectRecordingMode);
        android.widget.ScrollView scroll = new android.widget.ScrollView(this);
        scroll.setVerticalScrollBarEnabled(false);
        scroll.addView(panel);
        setContentView(scroll);
        position();
        service.refreshBubble();
    }

    private void position() {
        WindowManager.LayoutParams params = getWindow().getAttributes();
        params.gravity = Gravity.CENTER;
        params.width = Math.min(Ui.dp(this, 264), getResources().getDisplayMetrics().widthPixels - Ui.dp(this, 32));
        params.height = WindowManager.LayoutParams.WRAP_CONTENT;
        params.x = 0;
        params.y = 0;
        if (android.os.Build.VERSION.SDK_INT >= 30) {
            params.setFitInsetsTypes(0);
            params.layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS;
        }
        getWindow().setAttributes(params);
    }

    @Override
    public void onConfigurationChanged(android.content.res.Configuration configuration) {
        super.onConfigurationChanged(configuration);
        position();
    }

    @Override
    protected void onResume() {
        super.onResume();
        resumed = true;
        if (!completed && !started && service != null) {
            started = true;
            panel.post(() -> {
                if (!completed) {
                    service.beginCapture(session);
                }
            });
        }
    }

    void update(boolean recording, boolean processing, String mode) {
        if (panel != null) {
            panel.update(recording, processing, mode);
        }
    }

    void progress(int seconds) {
        if (panel != null) {
            panel.progress(seconds);
        }
    }

    void complete() {
        if (closing) {
            return;
        }
        closing = true;
        completed = true;
        main.removeCallbacksAndMessages(null);
        if (service != null) {
            service.recorderDismissed(this);
        }
        finish();
        overridePendingTransition(0, 0);
    }

    void delivered() {
        completed = true;
        panel.delivered();
        main.postDelayed(this::complete, 1200);
    }

    @Override
    protected void onPause() {
        resumed = false;
        if (!completed && service != null) {
            service.recorderStopped(this);
        } else if (completed) {
            complete();
        }
        super.onPause();
    }

    @Override
    protected void onDestroy() {
        main.removeCallbacksAndMessages(null);
        if (!completed && service != null) {
            service.recorderStopped(this);
        } else if (service != null) {
            service.recorderDismissed(this);
        }
        super.onDestroy();
    }
}
