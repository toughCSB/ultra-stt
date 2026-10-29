package com.ultrastt.androidvoice;

import android.content.Context;
import android.animation.ValueAnimator;
import android.content.res.ColorStateList;
import android.graphics.Color;
import android.graphics.drawable.GradientDrawable;
import android.os.Handler;
import android.os.Looper;
import android.view.Gravity;
import android.view.MotionEvent;
import android.view.ViewConfiguration;
import android.widget.FrameLayout;
import android.widget.ImageView;
import android.widget.ProgressBar;
import android.widget.TextView;
import android.view.View;

final class BubbleView extends FrameLayout {
    interface Gestures {
        void tap();
        void hold();
        void drag(float screenX, float screenY);
        void release(float screenX, float screenY);
        void cancel();
        void move(float deltaX, float deltaY);
        void finishMove(boolean cancelled);
    }

    private final Handler main = new Handler(Looper.getMainLooper());
    private final FrameLayout circle;
    private final ImageView microphone;
    private final View stop;
    private final TextView timer;
    private final ProgressBar spinner;
    private ValueAnimator pulse;
    private boolean recording;
    private boolean processing;
    private final Gestures gestures;
    private final BubbleGesture gesture = new BubbleGesture();
    private final Runnable longPress = () -> {
        if (gesture.state() == BubbleGesture.State.PRESSED) {
            performLongClick();
        }
    };

    BubbleView(Context context, Gestures gestures) {
        super(context);
        this.gestures = gestures;
        setClickable(true);
        setLongClickable(true);
        setFocusable(true);
        circle = new FrameLayout(context);
        addView(circle, new LayoutParams(Ui.dp(context, BubblePlacement.CIRCLE_SIZE),
                Ui.dp(context, BubblePlacement.CIRCLE_SIZE), Gravity.CENTER));
        microphone = new ImageView(context);
        microphone.setImageDrawable(ModeArtwork.load(context, R.drawable.mic_glyph));
        microphone.setColorFilter(Color.WHITE);
        microphone.setImportantForAccessibility(IMPORTANT_FOR_ACCESSIBILITY_NO);
        circle.addView(microphone, new LayoutParams(Ui.dp(context, 38), Ui.dp(context, 38), Gravity.CENTER));
        stop = new View(context);
        GradientDrawable stopShape = new GradientDrawable();
        stopShape.setColor(Color.WHITE);
        stopShape.setCornerRadius(Ui.dp(context, 3));
        stop.setBackground(stopShape);
        LayoutParams stopParams = new LayoutParams(Ui.dp(context, 16), Ui.dp(context, 16), Gravity.TOP | Gravity.CENTER_HORIZONTAL);
        stopParams.topMargin = Ui.dp(context, 10);
        circle.addView(stop, stopParams);
        timer = new TextView(context);
        timer.setTextColor(Color.WHITE);
        timer.setTextSize(10);
        timer.setGravity(Gravity.CENTER);
        LayoutParams timerParams = new LayoutParams(LayoutParams.MATCH_PARENT, Ui.dp(context, 16), Gravity.BOTTOM);
        timerParams.bottomMargin = Ui.dp(context, 5);
        circle.addView(timer, timerParams);
        spinner = new ProgressBar(context);
        spinner.setIndeterminateTintList(ColorStateList.valueOf(Color.WHITE));
        circle.addView(spinner, new LayoutParams(Ui.dp(context, 30), Ui.dp(context, 30), Gravity.CENTER));
        stop.setImportantForAccessibility(IMPORTANT_FOR_ACCESSIBILITY_NO);
        timer.setImportantForAccessibility(IMPORTANT_FOR_ACCESSIBILITY_NO);
        spinner.setImportantForAccessibility(IMPORTANT_FOR_ACCESSIBILITY_NO);
        update(false, false, "minimal");
    }

    @Override
    public boolean onTouchEvent(MotionEvent event) {
        switch (event.getActionMasked()) {
            case MotionEvent.ACTION_DOWN:
                gesture.down(event.getRawX(), event.getRawY());
                main.postDelayed(longPress, ViewConfiguration.getLongPressTimeout());
                return true;
            case MotionEvent.ACTION_MOVE:
                BubbleGesture.State state = gesture.move(event.getRawX(), event.getRawY(),
                        ViewConfiguration.get(getContext()).getScaledTouchSlop());
                if (state == BubbleGesture.State.SELECTING) {
                    gestures.drag(event.getRawX(), event.getRawY());
                } else if (state == BubbleGesture.State.MOVING) {
                    main.removeCallbacks(longPress);
                    gestures.move(gesture.deltaX(event.getRawX()), gesture.deltaY(event.getRawY()));
                }
                return true;
            case MotionEvent.ACTION_UP:
                main.removeCallbacks(longPress);
                BubbleGesture.State ending = gesture.move(event.getRawX(), event.getRawY(),
                        ViewConfiguration.get(getContext()).getScaledTouchSlop());
                if (ending == BubbleGesture.State.MOVING) {
                    gestures.move(gesture.deltaX(event.getRawX()), gesture.deltaY(event.getRawY()));
                }
                BubbleGesture.State finished = gesture.finish();
                if (finished == BubbleGesture.State.SELECTING) {
                    gestures.release(event.getRawX(), event.getRawY());
                } else if (finished == BubbleGesture.State.MOVING) {
                    gestures.finishMove(false);
                } else if (finished == BubbleGesture.State.PRESSED) {
                    performClick();
                }
                return true;
            case MotionEvent.ACTION_CANCEL:
            case MotionEvent.ACTION_POINTER_DOWN:
                cancelGesture();
                return true;
            default:
                return true;
        }
    }

    @Override
    public boolean performClick() {
        super.performClick();
        gestures.tap();
        return true;
    }

    @Override
    public boolean performLongClick() {
        if (gesture.state() == BubbleGesture.State.IDLE) {
            gesture.down(0, 0);
        }
        if (!gesture.hold()) {
            return false;
        }
        performHapticFeedback(android.view.HapticFeedbackConstants.LONG_PRESS);
        gestures.hold();
        return true;
    }

    boolean isInteracting() {
        return gesture.state() != BubbleGesture.State.IDLE;
    }

    void cancelGesture() {
        main.removeCallbacks(longPress);
        if (gesture.finish() == BubbleGesture.State.MOVING) {
            gestures.finishMove(true);
        }
        gestures.cancel();
    }

    void update(boolean recording, boolean processing, String mode) {
        boolean startPulse = recording && !processing;
        if (this.recording != startPulse) {
            if (pulse != null) {
                pulse.cancel();
                pulse = null;
            }
            circle.setScaleX(1f);
            circle.setScaleY(1f);
            if (startPulse) {
                timer.setText("00:00");
                pulse = ValueAnimator.ofFloat(0.91f, 1.05f);
                pulse.setDuration(650);
                pulse.setRepeatCount(ValueAnimator.INFINITE);
                pulse.setRepeatMode(ValueAnimator.REVERSE);
                pulse.addUpdateListener(animation -> {
                    float scale = (float) animation.getAnimatedValue();
                    circle.setScaleX(scale);
                    circle.setScaleY(scale);
                });
                pulse.start();
            }
        }
        this.recording = startPulse;
        this.processing = processing;
        GradientDrawable background = new GradientDrawable();
        background.setShape(GradientDrawable.OVAL);
        background.setColor(ModeArtwork.color(mode));
        background.setStroke(Ui.dp(getContext(), recording || processing ? 2 : 0), Color.WHITE);
        circle.setBackground(background);
        circle.setAlpha(recording || processing ? 1f : BubblePlacement.IDLE_ALPHA);
        microphone.setVisibility(recording || processing ? GONE : VISIBLE);
        stop.setVisibility(startPulse ? VISIBLE : GONE);
        timer.setVisibility(startPulse ? VISIBLE : GONE);
        spinner.setVisibility(processing ? VISIBLE : GONE);
        String state = processing ? "음성 처리 중 · 잠시 기다려 주세요 · 길게 누른 채 드래그해 모드 변경"
                : recording ? "녹음 중 · 눌러 녹음 종료 · 길게 누른 채 드래그해 모드 변경"
                : "Ultra STT · 눌러 녹음 시작 · 바로 드래그해 위치 이동 · 길게 누르면 진동 후 모드 선택";
        setContentDescription(Modes.LABELS[Modes.index(mode)] + " · " + state);
    }

    void progress(int seconds) {
        if (recording && !processing) {
            timer.setText(String.format(java.util.Locale.ROOT, "%02d:%02d", seconds / 60, seconds % 60));
            setContentDescription("녹음 중 " + seconds + "초 · 눌러 종료 · 길게 눌러 모드 변경");
        }
    }

    @Override
    protected void onDetachedFromWindow() {
        cancelGesture();
        if (pulse != null) {
            pulse.cancel();
            pulse = null;
        }
        super.onDetachedFromWindow();
    }
}
