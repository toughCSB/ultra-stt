package com.ultrastt.androidvoice;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.drawable.BitmapDrawable;
import android.graphics.drawable.Drawable;
import android.util.SparseArray;

final class ModeArtwork {
    private static final SparseArray<Bitmap> BITMAPS = new SparseArray<>();
    private static final int[] RESOURCES = {R.drawable.mode_original, R.drawable.mode_minimal,
            R.drawable.mode_clean, R.drawable.mode_structured, R.drawable.mode_professional};
    private static final int[] COLORS = {0xFF20A044, 0xFF008C78, 0xFF007EA8, 0xFF2563EB, 0xFF5046C8};

    static int resource(String mode) {
        return RESOURCES[Modes.index(mode)];
    }

    static int color(String mode) {
        return COLORS[Modes.index(mode)];
    }

    static Drawable load(Context context, int resource) {
        Bitmap bitmap = BITMAPS.get(resource);
        if (bitmap == null) {
            BitmapFactory.Options options = new BitmapFactory.Options();
            options.inSampleSize = 8;
            bitmap = BitmapFactory.decodeResource(context.getResources(), resource, options);
            BITMAPS.put(resource, bitmap);
        }
        return new BitmapDrawable(context.getResources(), bitmap);
    }

    private ModeArtwork() {}
}
