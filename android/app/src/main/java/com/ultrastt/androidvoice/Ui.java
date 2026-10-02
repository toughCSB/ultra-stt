package com.ultrastt.androidvoice;

import android.content.Context;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;

final class Ui {
    static final int BACKGROUND = Color.rgb(7, 16, 12);
    static final int PANEL = Color.rgb(15, 37, 27);
    static final int GREEN = Color.rgb(0, 227, 155);
    static final int TEXT = Color.rgb(234, 246, 239);
    static final int MUTED = Color.rgb(159, 185, 172);

    static int dp(Context context, int value) {
        return Math.round(value * context.getResources().getDisplayMetrics().density);
    }

    static LinearLayout column(Context context, int padding) {
        LinearLayout layout = new LinearLayout(context);
        layout.setOrientation(LinearLayout.VERTICAL);
        layout.setBackgroundColor(BACKGROUND);
        int spacing = dp(context, padding);
        layout.setPadding(spacing, spacing, spacing, spacing);
        return layout;
    }

    static TextView text(Context context, String value, int size, boolean strong) {
        TextView label = new TextView(context);
        label.setText(value);
        label.setTextSize(size);
        label.setTextColor(strong ? TEXT : MUTED);
        if (strong) {
            label.setTypeface(Typeface.DEFAULT, Typeface.BOLD);
        }
        label.setPadding(0, dp(context, 6), 0, dp(context, 6));
        return label;
    }

    static Button button(Context context, String label) {
        Button button = new Button(context);
        button.setText(label);
        button.setAllCaps(false);
        button.setTextSize(14);
        button.setMinHeight(dp(context, 48));
        button.setTextColor(TEXT);
        button.setBackground(background(context, PANEL, false));
        button.setPadding(dp(context, 12), dp(context, 6), dp(context, 12), dp(context, 6));
        return button;
    }

    static GradientDrawable background(Context context, int color, boolean selected) {
        GradientDrawable drawable = new GradientDrawable();
        drawable.setColor(color);
        drawable.setCornerRadius(dp(context, 14));
        drawable.setStroke(dp(context, 1), selected ? GREEN : Color.rgb(35, 67, 51));
        return drawable;
    }

    static LinearLayout.LayoutParams block(Context context) {
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(-1, -2);
        params.setMargins(0, dp(context, 5), 0, dp(context, 5));
        return params;
    }

    private Ui() {}
}
