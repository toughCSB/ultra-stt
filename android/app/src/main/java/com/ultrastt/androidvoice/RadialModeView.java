package com.ultrastt.androidvoice;

import android.content.Context;
import android.graphics.Color;
import android.graphics.drawable.GradientDrawable;
import android.view.Gravity;
import android.widget.FrameLayout;
import android.widget.ImageButton;
import android.widget.TextView;

import java.util.function.IntConsumer;

final class RadialModeView extends FrameLayout {
    private final ImageButton[] buttons = new ImageButton[5];
    private final TextView label;
    private final int selected;
    private int highlighted = -1;

    RadialModeView(Context context, float originX, float originY, String mode, IntConsumer choose) {
        super(context);
        selected = Modes.index(mode);
        setClipChildren(false);
        setClipToPadding(false);
        for (int index = 0; index < buttons.length; index++) {
            final int position = index;
            ImageButton button = new ImageButton(context);
            button.setImageDrawable(ModeArtwork.load(context, ModeArtwork.resource(Modes.IDS[index])));
            button.setColorFilter(Color.WHITE);
            button.setScaleType(android.widget.ImageView.ScaleType.FIT_CENTER);
            int padding = Ui.dp(context, 9);
            button.setPadding(padding, padding, padding, padding);
            button.setContentDescription(Modes.LABELS[index] + (index == selected ? " · 현재 선택" : ""));
            button.setOnClickListener(view -> choose.accept(position));
            float density = context.getResources().getDisplayMetrics().density;
            float centerX = originX + (float) RadialGeometry.x(index) * density;
            float centerY = originY + (float) RadialGeometry.y(index) * density;
            LayoutParams params = new LayoutParams(Ui.dp(context, 48), Ui.dp(context, 48));
            params.leftMargin = Math.round(centerX - Ui.dp(context, 24));
            params.topMargin = Math.round(centerY - Ui.dp(context, 24));
            addView(button, params);
            buttons[index] = button;
            button.setTranslationX(originX - centerX);
            button.setTranslationY(originY - centerY);
            button.setAlpha(0f);
            button.animate().translationX(0).translationY(0).alpha(1f).setDuration(140).start();
        }
        label = Ui.text(context, Modes.LABELS[selected], 13, true);
        label.setGravity(Gravity.CENTER);
        LayoutParams textParams = new LayoutParams(Ui.dp(context, 88), Ui.dp(context, 42));
        textParams.leftMargin = Math.round(originX - Ui.dp(context, 96));
        textParams.topMargin = Math.round(originY - Ui.dp(context, 21));
        addView(label, textParams);
        highlight(-1);
    }

    void highlight(int index) {
        if (index != highlighted && index >= 0) {
            performHapticFeedback(android.view.HapticFeedbackConstants.CLOCK_TICK);
        }
        highlighted = index;
        label.setText(index >= 0 ? Modes.LABELS[index] : Modes.LABELS[selected]);
        for (int position = 0; position < buttons.length; position++) {
            boolean active = position == index;
            GradientDrawable background = new GradientDrawable();
            background.setShape(GradientDrawable.OVAL);
            background.setColor(ModeArtwork.color(Modes.IDS[position]));
            background.setStroke(Ui.dp(getContext(), active ? 3 : position == selected ? 2 : 0), Color.WHITE);
            buttons[position].setBackground(background);
            buttons[position].setScaleX(active ? 1.12f : 1f);
            buttons[position].setScaleY(active ? 1.12f : 1f);
            buttons[position].setSelected(position == selected);
        }
    }
}
