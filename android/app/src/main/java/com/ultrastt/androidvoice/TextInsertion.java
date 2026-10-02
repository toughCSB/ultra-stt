package com.ultrastt.androidvoice;

final class TextInsertion {
    static String editorText(CharSequence value, boolean showingHint) {
        return showingHint || value == null ? "" : value.toString();
    }

    static int editorSelection(String text, int position) {
        return text.isEmpty() && position == -1 ? 0 : position;
    }

    static String replaceSelection(String original, int start, int end, String inserted) {
        if (original == null || inserted == null || start < 0 || end < 0
                || start > original.length() || end > original.length()) {
            throw new IllegalArgumentException("커서 위치를 확인할 수 없어 입력하지 않았습니다.");
        }
        int left = Math.min(start, end);
        int right = Math.max(start, end);
        if (splitsSurrogate(original, left) || splitsSurrogate(original, right)) {
            throw new IllegalArgumentException("안전하게 입력할 수 없는 커서 위치입니다.");
        }
        return original.substring(0, left) + inserted + original.substring(right);
    }

    private static boolean splitsSurrogate(String text, int position) {
        return position > 0 && position < text.length()
                && Character.isHighSurrogate(text.charAt(position - 1))
                && Character.isLowSurrogate(text.charAt(position));
    }

    private TextInsertion() {}
}
