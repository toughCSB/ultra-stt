package com.ultrastt.androidvoice;

import android.app.Activity;
import android.os.Bundle;
import android.graphics.Color;
import android.text.InputType;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.TextView;

public final class EditorActivity extends Activity {
    private boolean paused;
    private boolean editorVisible = true;
    static boolean keyboardTransition;
    static java.lang.ref.WeakReference<EditText> activeEditor = new java.lang.ref.WeakReference<>(null);

    @Override
    protected void onResume() {
        super.onResume();
        paused = false;
    }

    @Override
    protected void onPause() {
        paused = true;
        super.onPause();
    }

    @Override
    public void onCreate(Bundle state) {
        super.onCreate(state);
        editorVisible = !getIntent().getBooleanExtra("lateVisible", false);
        keyboardTransition = false;
        LinearLayout content = new LinearLayout(this);
        content.setOrientation(LinearLayout.VERTICAL);
        content.setPadding(24, 100, 24, 24);
        content.setBackgroundColor(Color.WHITE);
        TextView title = new TextView(this);
        title.setText("Ultra STT QA · 외부 앱 커서 테스트");
        title.setTextSize(20);
        title.setTextColor(Color.BLACK);
        content.addView(title);
        EditText editor = new EditText(this) {
            @Override
            public android.view.accessibility.AccessibilityNodeProvider getAccessibilityNodeProvider() {
                if (!getIntent().getBooleanExtra("staleFocus", false)) {
                    return super.getAccessibilityNodeProvider();
                }
                EditText host = this;
                return new android.view.accessibility.AccessibilityNodeProvider() {
                    @Override
                    public android.view.accessibility.AccessibilityNodeInfo createAccessibilityNodeInfo(int identifier) {
                        if (identifier != HOST_VIEW_ID && identifier != 1) {
                            return null;
                        }
                        android.view.accessibility.AccessibilityNodeInfo node = android.view.accessibility.AccessibilityNodeInfo.obtain();
                        host.onInitializeAccessibilityNodeInfo(node);
                        if (identifier == HOST_VIEW_ID) {
                            node.setSource(host);
                            node.setEditable(false);
                            node.setFocused(false);
                            node.addChild(host, 1);
                        } else {
                            node.setSource(host, 1);
                            node.setParent(host);
                            if (getIntent().getBooleanExtra("unfocused", false)) {
                                node.setFocused(false);
                            }
                        }
                        return node;
                    }

                    @Override
                    public android.view.accessibility.AccessibilityNodeInfo findFocus(int focus) {
                        return createAccessibilityNodeInfo(HOST_VIEW_ID);
                    }

                    @Override
                    public boolean performAction(int identifier, int action, Bundle options) {
                        return host.performAccessibilityAction(action, options);
                    }
                };
            }

            @Override
            public android.view.inputmethod.InputConnection onCreateInputConnection(android.view.inputmethod.EditorInfo info) {
                android.view.inputmethod.InputConnection connection = super.onCreateInputConnection(info);
                if (connection == null || android.os.Build.VERSION.SDK_INT < 31 || !getIntent().getBooleanExtra("keyboardBlur", false)) {
                    return connection;
                }
                return new android.view.inputmethod.InputConnectionWrapper(connection, false) {
                    @Override
                    public android.view.inputmethod.SurroundingText getSurroundingText(int before, int after, int flags) {
                        android.view.inputmethod.SurroundingText text = super.getSurroundingText(before, after, flags);
                        return text == null ? null : new android.view.inputmethod.SurroundingText(
                                text.getText(), text.getSelectionStart(), text.getSelectionEnd(), -1);
                    }
                };
            }

            @Override
            public void onInitializeAccessibilityNodeInfo(android.view.accessibility.AccessibilityNodeInfo info) {
                super.onInitializeAccessibilityNodeInfo(info);
                if (!editorVisible) {
                    info.setVisibleToUser(false);
                }
                if (paused && getIntent().getBooleanExtra("paused", false)) {
                    info.setFocused(false);
                }
                if (getIntent().getBooleanExtra("placeholder", false)) {
                    info.setText(getText().length() == 0 ? "메시지" : getText());
                    info.setShowingHintText(false);
                    info.setTextSelection(-1, -1);
                }
                if (getIntent().getBooleanExtra("keyboardBlur", false)) {
                    info.setText(getText());
                    info.setShowingHintText(false);
                }
            }
        };
        editor.setId(android.R.id.edit);
        editor.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_MULTI_LINE);
        if (getIntent().getBooleanExtra("password", false)) {
            editor.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD);
        }
        editor.setHint("QA 전용 빈 입력창 · 실제 대화가 아닙니다");
        editor.setTextColor(Color.BLACK);
        editor.setHintTextColor(Color.GRAY);
        if (getIntent().getBooleanExtra("content", false)) {
            editor.setText("앞 뒤");
            editor.setSelection(2);
        }
        activeEditor = new java.lang.ref.WeakReference<>(editor);
        content.addView(editor, new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, 240));
        setContentView(content);
        if (android.os.Build.VERSION.SDK_INT >= 30 && getIntent().getBooleanExtra("keyboardBlur", false)) {
            content.setFocusableInTouchMode(true);
            boolean[] keyboardState = {false, false};
            content.setOnApplyWindowInsetsListener((view, insets) -> {
                boolean visible = insets.isVisible(android.view.WindowInsets.Type.ime());
                if (keyboardState[0] && !visible && !keyboardState[1]) {
                    keyboardState[1] = true;
                    keyboardTransition = true;
                    content.requestFocus();
                }
                keyboardState[0] = visible;
                return insets;
            });
        }
        editor.requestFocus();
        if (!editorVisible) {
            editor.postDelayed(() -> {
                editorVisible = true;
                editor.sendAccessibilityEvent(android.view.accessibility.AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED);
            }, 2500);
        }
        if (getIntent().getBooleanExtra("busy", false)) {
            android.os.Handler updates = new android.os.Handler(android.os.Looper.getMainLooper());
            long end = android.os.SystemClock.uptimeMillis() + 8000;
            updates.post(new Runnable() {
                private int index;

                @Override
                public void run() {
                    if (android.os.SystemClock.uptimeMillis() < end) {
                        int[] events = {android.view.accessibility.AccessibilityEvent.TYPE_VIEW_FOCUSED,
                                android.view.accessibility.AccessibilityEvent.TYPE_VIEW_CLICKED,
                                android.view.accessibility.AccessibilityEvent.TYPE_VIEW_TEXT_SELECTION_CHANGED};
                        editor.sendAccessibilityEvent(events[index++ % events.length]);
                        updates.postDelayed(this, 30);
                    }
                }
            });
        }
    }
}
