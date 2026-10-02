package com.ultrastt.androidvoice;

final class Modes {
    static final String[] IDS = {"original", "minimal", "clean", "structured", "professional"};
    static final String[] LABELS = {"원문", "최소", "깔끔하게", "구조화", "전문적으로"};

    static int index(String value) {
        for (int index = 0; index < IDS.length; index++) {
            if (IDS[index].equals(value)) {
                return index;
            }
        }
        return 1;
    }

    private Modes() {}
}
