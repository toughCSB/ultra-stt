package com.ultrastt.androidvoice;

import android.animation.ValueAnimator;
import android.content.Context;
import android.content.res.ColorStateList;
import android.graphics.Typeface;
import android.graphics.drawable.Drawable;
import android.graphics.drawable.GradientDrawable;
import android.view.Gravity;
import android.view.View;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.ProgressBar;
import android.widget.TextView;

final class RecordingPanelView extends LinearLayout {
    private final TextView status;
    private final TextView timer;
    private final TextView mode;
    private final TextView action;
    private final TextView cancel;
    private final ImageView microphone;
    private final ProgressBar spinner;
    private final LinearLayout choices;
    private ValueAnimator pulse;
    private boolean processing;
    private boolean delivered;
    private boolean recording;

    RecordingPanelView(Context context, Drawable artwork, Drawable wordmark, Runnable stop, Runnable dismiss,
            java.util.function.Consumer<String> selectMode) {
        super(context);
        setOrientation(VERTICAL);
        setPadding(dp(16), dp(12), dp(16), dp(12));
        GradientDrawable glass = new GradientDrawable(GradientDrawable.Orientation.TL_BR,
                new int[]{0xCC1C775C, 0xD2083E34});
        glass.setCornerRadius(dp(24));
        glass.setStroke(dp(1), 0x804EF5BA);
        setBackground(glass);

        LinearLayout heading = new LinearLayout(context);
        heading.setGravity(Gravity.CENTER_VERTICAL);
        ImageView brand = new ImageView(context);
        brand.setImageDrawable(wordmark);
        brand.setScaleType(ImageView.ScaleType.FIT_START);
        brand.setAdjustViewBounds(true);
        brand.setPadding(0, dp(7), dp(24), dp(7));
        brand.setContentDescription("Ultra STT");
        heading.addView(brand, new LayoutParams(0, dp(36), 1));
        cancel = label("✕", 19, 0xFFD7F7E9);
        cancel.setGravity(Gravity.CENTER);
        cancel.setContentDescription("음성 입력 취소");
        cancel.setOnClickListener(view -> dismiss.run());
        heading.addView(cancel, new LayoutParams(dp(48), dp(36)));
        addView(heading);

        LinearLayout body = new LinearLayout(context);
        body.setGravity(Gravity.CENTER_VERTICAL);
        microphone = new ImageView(context);
        microphone.setImageDrawable(artwork);
        microphone.setColorFilter(0xFFB3FFDF);
        microphone.setPadding(dp(10), dp(10), dp(10), dp(10));
        microphone.setBackground(pill(0x483FE1A1, 30));
        microphone.setImportantForAccessibility(IMPORTANT_FOR_ACCESSIBILITY_NO);
        body.addView(microphone, new LayoutParams(dp(48), dp(48)));
        spinner = new ProgressBar(context);
        spinner.setIndeterminateTintList(ColorStateList.valueOf(0xFF7AFFC5));
        spinner.setVisibility(GONE);
        body.addView(spinner, new LayoutParams(dp(48), dp(48)));
        LinearLayout labels = new LinearLayout(context);
        labels.setOrientation(VERTICAL);
        status = label("준비 중", 18, 0xFFF1FFF7);
        status.setAccessibilityLiveRegion(ACCESSIBILITY_LIVE_REGION_POLITE);
        labels.addView(status);
        timer = label("00:00", 22, 0xFFB1FADB);
        timer.setTypeface(Typeface.MONOSPACE);
        timer.setContentDescription("녹음 시간 0초");
        labels.addView(timer);
        LayoutParams labelParams = new LayoutParams(0, -2, 1);
        labelParams.leftMargin = dp(12);
        body.addView(labels, labelParams);
        LayoutParams bodyParams = new LayoutParams(-1, -2);
        bodyParams.setMargins(0, dp(6), 0, dp(10));
        addView(body, bodyParams);

        mode = label("최소 모드  ▾", 14, 0xFFC7FFE4);
        mode.setGravity(Gravity.CENTER);
        mode.setBackground(pill(0x382CE0A4, 16));
        choices = new LinearLayout(context);
        mode.setOnClickListener(view -> {
            if (!processing && !delivered) {
                choices.setVisibility(choices.getVisibility() == VISIBLE ? GONE : VISIBLE);
            }
        });
        addView(mode, new LayoutParams(-1, dp(42)));
        choices.setOrientation(VERTICAL);
        choices.setVisibility(GONE);
        for (int index = 0; index < Modes.IDS.length; index++) {
            String selected = Modes.IDS[index];
            TextView choice = label(Modes.LABELS[index], 14, 0xFFE0FFF0);
            choice.setGravity(Gravity.CENTER);
            choice.setContentDescription(Modes.LABELS[index] + " 모드 선택");
            choice.setOnClickListener(view -> {
                if (!processing && !delivered) {
                    choices.setVisibility(GONE);
                    selectMode.accept(selected);
                }
            });
            choices.addView(choice, new LayoutParams(-1, dp(42)));
        }
        addView(choices);

        action = label("■  녹음 종료", 15, 0xFF043C2A);
        action.setGravity(Gravity.CENTER);
        action.setBackground(pill(0xFF64F0B2, 18));
        action.setOnClickListener(view -> {
            if (recording && !processing && !delivered) {
                stop.run();
            }
        });
        LayoutParams actionParams = new LayoutParams(-1, dp(48));
        actionParams.topMargin = dp(8);
        addView(action, actionParams);
        update(false, false, "minimal");
    }

    void update(boolean active, boolean busy, String selectedMode) {
        if (delivered) {
            return;
        }
        recording = active;
        processing = busy;
        status.setText(busy ? "변환 중…" : active ? "듣고 있어요" : "준비 중…");
        mode.setText(Modes.LABELS[Modes.index(selectedMode)] + " 모드" + (busy ? "" : "  ▾"));
        mode.setEnabled(!busy);
        action.setEnabled(active && !busy);
        action.setText(busy ? "텍스트로 다듬고 있어요" : "■  녹음 종료");
        action.setAlpha(busy ? 0.65f : 1f);
        spinner.setVisibility(busy ? VISIBLE : GONE);
        microphone.setVisibility(busy ? GONE : VISIBLE);
        if (busy) {
            choices.setVisibility(GONE);
        }
        animateMicrophone(active && !busy);
    }

    void progress(int seconds) {
        if (recording && !processing && !delivered) {
            timer.setText(String.format(java.util.Locale.ROOT, "%02d:%02d", seconds / 60, seconds % 60));
            timer.setContentDescription("녹음 시간 " + seconds + "초");
        }
    }

    void delivered() {
        delivered = true;
        recording = false;
        processing = false;
        animateMicrophone(false);
        status.setText("전송 완료");
        action.setText("✓  입력창으로 전달했어요");
        action.setAlpha(1f);
        action.setEnabled(false);
        mode.setEnabled(false);
        cancel.setEnabled(false);
        cancel.setVisibility(INVISIBLE);
        choices.setVisibility(GONE);
        spinner.setVisibility(GONE);
        microphone.setVisibility(VISIBLE);
    }

    private void animateMicrophone(boolean active) {
        if (active && pulse == null) {
            pulse = ValueAnimator.ofFloat(0.9f, 1.04f);
            pulse.setDuration(650);
            pulse.setRepeatCount(ValueAnimator.INFINITE);
            pulse.setRepeatMode(ValueAnimator.REVERSE);
            pulse.addUpdateListener(animation -> {
                float scale = (float) animation.getAnimatedValue();
                microphone.setScaleX(scale);
                microphone.setScaleY(scale);
            });
            pulse.start();
        } else if (!active && pulse != null) {
            pulse.cancel();
            pulse = null;
            microphone.setScaleX(1);
            microphone.setScaleY(1);
        }
    }

    private int dp(int size) {
        return Ui.dp(getContext(), size);
    }

    private TextView label(String value, int size, int color) {
        TextView view = new TextView(getContext());
        view.setText(value);
        view.setTextSize(size);
        view.setTextColor(color);
        view.setTypeface(Typeface.DEFAULT, Typeface.BOLD);
        view.setGravity(Gravity.CENTER_VERTICAL);
        return view;
    }

    private GradientDrawable pill(int color, int radius) {
        GradientDrawable shape = new GradientDrawable();
        shape.setColor(color);
        shape.setCornerRadius(dp(radius));
        return shape;
    }

    @Override
    protected void onDetachedFromWindow() {
        animateMicrophone(false);
        super.onDetachedFromWindow();
    }
}
