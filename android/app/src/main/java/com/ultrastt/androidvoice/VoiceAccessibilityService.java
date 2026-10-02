package com.ultrastt.androidvoice;

import android.Manifest;
import android.accessibilityservice.AccessibilityService;
import android.accessibilityservice.AccessibilityServiceInfo;
import android.accessibilityservice.InputMethod;
import android.app.KeyguardManager;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.graphics.PixelFormat;
import android.graphics.Rect;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.PowerManager;
import android.view.Gravity;
import android.view.WindowManager;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import android.view.accessibility.AccessibilityWindowInfo;
import android.view.inputmethod.EditorInfo;
import android.widget.Toast;

import java.util.List;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public final class VoiceAccessibilityService extends AccessibilityService {
    private static java.lang.ref.WeakReference<VoiceAccessibilityService> instance = new java.lang.ref.WeakReference<>(null);
    private final Handler main = new Handler(Looper.getMainLooper());
    private final EditorGuard guard = new EditorGuard();
    private final ExecutorService audioWorker = Executors.newSingleThreadExecutor();
    private final ExecutorService networkWorker = Executors.newSingleThreadExecutor();
    private final Runnable update = this::updateTarget;
    private Preferences preferences;
    private WindowManager windows;
    private BubbleView bubble;
    private RadialModeView palette;
    private AccessibilityNodeInfo focused;
    private AccessibilityNodeInfo captured;
    private String original;
    private int selectionStart;
    private int selectionEnd;
    private AudioCapture capture;
    private TranscriptionRequest request;
    private RecordingActivity recordingActivity;
    private boolean sending;
    private boolean writing;
    private boolean destroyed;
    private long session;
    private int anchorX;
    private int anchorY;
    private int displayHeight;
    private float radialCenterX;
    private float radialCenterY;

    static VoiceAccessibilityService active() {
        return instance.get();
    }

    @Override
    protected void onServiceConnected() {
        super.onServiceConnected();
        instance = new java.lang.ref.WeakReference<>(this);
        preferences = new Preferences(this);
        windows = (WindowManager) getSystemService(WINDOW_SERVICE);
        if (Build.VERSION.SDK_INT >= 33) {
            AccessibilityServiceInfo info = getServiceInfo();
            info.flags |= AccessibilityServiceInfo.FLAG_INPUT_METHOD_EDITOR;
            setServiceInfo(info);
        }
        scheduleUpdate();
    }

    @android.annotation.TargetApi(33)
    @Override
    public InputMethod onCreateInputMethod() {
        return new InputMethod(this) {
            @Override
            public void onStartInput(EditorInfo info, boolean restarting) {
                super.onStartInput(info, restarting);
                if (guard.isActive() && captured != null && !samePackage(captured, info.packageName)) {
                    cancelWork("입력창이 바뀌어 취소했습니다.");
                }
                scheduleUpdate();
            }

            @Override
            public void onFinishInput() {
                if (guard.isActive() && !writing) {
                    cancelWork("입력창이 닫혀 취소했습니다.");
                }
                super.onFinishInput();
                scheduleUpdate();
            }

            @Override
            public void onUpdateSelection(int oldStart, int oldEnd, int newStart, int newEnd,
                    int candidatesStart, int candidatesEnd) {
                super.onUpdateSelection(oldStart, oldEnd, newStart, newEnd, candidatesStart, candidatesEnd);
                if (guard.isActive() && !writing && (oldStart != newStart || oldEnd != newEnd)) {
                    cancelWork("커서가 이동해 취소했습니다.");
                }
            }
        };
    }

    @Override
    public void onAccessibilityEvent(AccessibilityEvent event) {
        if (destroyed || preferences == null) {
            return;
        }
        if (guard.isActive() && !writing && captured != null
                && (event.getEventType() == AccessibilityEvent.TYPE_VIEW_TEXT_CHANGED
                || event.getEventType() == AccessibilityEvent.TYPE_VIEW_TEXT_SELECTION_CHANGED)) {
            AccessibilityNodeInfo source = event.getSource();
            if (source != null && captured.equals(source) && !unchangedTarget()) {
                cancelWork("입력 내용이나 커서가 바뀌어 취소했습니다.");
            }
        }
        scheduleUpdate();
    }

    private void scheduleUpdate() {
        main.removeCallbacks(update);
        main.postDelayed(update, 100);
    }

    private boolean samePackage(AccessibilityNodeInfo node, String packageName) {
        return node.getPackageName() != null && node.getPackageName().toString().equals(packageName);
    }

    private boolean eligible(AccessibilityNodeInfo node) {
        if (node == null || !node.isEditable() || !node.isFocused() || !node.isVisibleToUser()
                || !node.isEnabled() || node.isPassword()) {
            return false;
        }
        if (samePackage(node, getPackageName())) {
            return (getPackageName() + ":id/voice_test_editor").equals(node.getViewIdResourceName());
        }
        return true;
    }

    private AccessibilityNodeInfo findEditor() {
        PowerManager power = (PowerManager) getSystemService(POWER_SERVICE);
        KeyguardManager keyguard = (KeyguardManager) getSystemService(KEYGUARD_SERVICE);
        if (!power.isInteractive() || keyguard.isKeyguardLocked()) {
            return null;
        }
        List<AccessibilityWindowInfo> visibleWindows = getWindows();
        for (AccessibilityWindowInfo window : visibleWindows) {
            if (window.isActive() && window.getType() == AccessibilityWindowInfo.TYPE_SYSTEM) {
                return null;
            }
        }
        AccessibilityNodeInfo node = findFocus(AccessibilityNodeInfo.FOCUS_INPUT);
        if (eligible(node)) {
            return node;
        }
        for (AccessibilityWindowInfo window : visibleWindows) {
            if (window.getType() != AccessibilityWindowInfo.TYPE_APPLICATION) {
                continue;
            }
            AccessibilityNodeInfo root = window.getRoot();
            if (root != null) {
                node = root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT);
                if (eligible(node)) {
                    return node;
                }
            }
        }
        return null;
    }

    private void updateTarget() {
        if (destroyed || preferences == null) {
            return;
        }
        AccessibilityNodeInfo editor = preferences.floatingEnabled() ? findEditor() : null;
        if (guard.isActive() && (editor == null || captured == null || !captured.equals(editor) || !unchangedTarget())) {
            cancelWork("입력창이 바뀌어 취소했습니다.");
        }
        focused = editor;
        if (editor == null) {
            hideBubble();
            closePalette();
            return;
        }
        position(editor);
        if (recordingActivity == null && !guard.isActive()) {
            showBubble();
        }
    }

    private void position(AccessibilityNodeInfo editor) {
        Rect display;
        if (Build.VERSION.SDK_INT >= 30) {
            display = windows.getCurrentWindowMetrics().getBounds();
        } else {
            android.util.DisplayMetrics metrics = new android.util.DisplayMetrics();
            windows.getDefaultDisplay().getRealMetrics(metrics);
            display = new Rect(0, 0, metrics.widthPixels, metrics.heightPixels);
        }
        Rect field = new Rect();
        displayHeight = display.height();
        editor.getBoundsInScreen(field);
        int keyboardTop = display.height();
        boolean keyboardVisible = false;
        for (AccessibilityWindowInfo window : getWindows()) {
            if (window.getType() == AccessibilityWindowInfo.TYPE_INPUT_METHOD) {
                Rect bounds = new Rect();
                window.getBoundsInScreen(bounds);
                if (!bounds.isEmpty()) {
                    keyboardTop = Math.min(keyboardTop, bounds.top);
                    keyboardVisible = true;
                }
            }
        }
        anchorX = Math.max(Ui.dp(this, 8), display.width() - Ui.dp(this, 64));
        int desired = keyboardVisible ? keyboardTop - Ui.dp(this, 64) : field.bottom + Ui.dp(this, 8);
        anchorY = Math.max(Ui.dp(this, 32), Math.min(desired, display.height() - Ui.dp(this, 96)));
        if (bubble != null) {
            windows.updateViewLayout(bubble, overlayParams(Ui.dp(this, 56), Ui.dp(this, 56), anchorX, anchorY));
        }
        if (recordingActivity != null) {
            recordingActivity.position(anchorX, anchorY);
        }
    }

    private WindowManager.LayoutParams overlayParams(int width, int height, int x, int y) {
        WindowManager.LayoutParams params = new WindowManager.LayoutParams(width, height,
                WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
                WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE | WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL
                        | WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
                PixelFormat.TRANSLUCENT);
        params.gravity = Gravity.TOP | Gravity.LEFT;
        params.x = x;
        params.y = y;
        if (Build.VERSION.SDK_INT >= 30) {
            params.setFitInsetsTypes(0);
            params.layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS;
        }
        return params;
    }

    private void showBubble() {
        if (bubble != null || focused == null) {
            return;
        }
        bubble = new BubbleView(this, gestures());
        try {
            windows.addView(bubble, overlayParams(Ui.dp(this, 56), Ui.dp(this, 56), anchorX, anchorY));
            refreshBubble();
        } catch (WindowManager.BadTokenException error) {
            bubble = null;
        }
    }

    private void hideBubble() {
        if (bubble != null) {
            windows.removeView(bubble);
            bubble = null;
        }
    }

    void toggle() {
        closePalette();
        if (sending) {
            notice("음성을 처리하고 있습니다. 잠시 기다려 주세요.");
        } else if (capture != null) {
            sending = true;
            capture.stop();
            refreshBubble();
        } else if (!guard.isActive()) {
            prepareRecording();
        }
    }

    private void prepareRecording() {
        AccessibilityNodeInfo editor = findEditor();
        if (!eligible(editor)) {
            notice("입력창을 먼저 눌러 주세요. 비밀번호 칸에서는 사용할 수 없습니다.");
            return;
        }
        if (checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
            notice("Ultra STT 앱에서 마이크 권한을 먼저 허용해 주세요.");
            return;
        }
        try {
            if (!Endpoint.validToken(preferences.token())) {
                notice("PC 연결 설정이 아직 없습니다. Ultra STT 앱에서 연결 상태를 확인해 주세요.");
                return;
            }
        } catch (java.security.GeneralSecurityException error) {
            notice("서버 연결 정보를 다시 설정해 주세요.");
            return;
        }
        captured = AccessibilityNodeInfo.obtain(editor);
        original = TextInsertion.editorText(editor.getText(), editor.isShowingHintText());
        selectionStart = TextInsertion.editorSelection(original, editor.getTextSelectionStart());
        selectionEnd = TextInsertion.editorSelection(original, editor.getTextSelectionEnd());
        if (selectionStart < 0 || selectionEnd < 0 || selectionStart > original.length()
                || selectionEnd > original.length() || original.length() > 200000) {
            releaseTarget();
            notice("이 입력창의 커서를 안전하게 확인할 수 없어 녹음하지 않았습니다.");
            return;
        }
        session = guard.begin();
        hideBubble();
        Intent recorder = new Intent(this, RecordingActivity.class);
        recorder.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_NO_ANIMATION);
        recorder.putExtra("session", session);
        recorder.putExtra("x", anchorX);
        recorder.putExtra("y", anchorY);
        try {
            startActivity(recorder);
            long expected = session;
            main.postDelayed(() -> {
                if (guard.accepts(expected) && recordingActivity == null) {
                    cancelWork("녹음 창을 열지 못했습니다. 앱을 한 번 열고 다시 시도해 주세요.");
                }
            }, 2500);
        } catch (RuntimeException error) {
            cancelWork("녹음 창을 열지 못했습니다. 접근성 설정을 확인해 주세요.");
        }
    }

    boolean attachRecorder(RecordingActivity activity, long expected) {
        if (!guard.accepts(expected) || !unchangedTarget()) {
            return false;
        }
        recordingActivity = activity;
        return true;
    }

    void beginCapture(long expected) {
        if (!guard.accepts(expected) || capture != null || sending || !unchangedTarget()) {
            return;
        }
        capture = new AudioCapture(new AudioCapture.Listener() {
            @Override
            public void onAudio(byte[] wav) {
                if (!guard.accepts(expected)) {
                    return;
                }
                capture = null;
                sending = true;
                refreshBubble();
                try {
                    request = new TranscriptionRequest(preferences.endpoint(), preferences.serverAddress(), preferences.token(),
                            preferences.mode(), preferences.polishEnabled(), wav, new TranscriptionRequest.Listener() {
                        @Override
                        public void onResult(String text) {
                            if (guard.accepts(expected)) {
                                insert(text);
                            }
                        }

                        @Override
                        public void onError(String message) {
                            if (guard.accepts(expected)) {
                                cancelWork(message);
                            }
                        }
                    });
                    networkWorker.execute(request);
                } catch (java.security.GeneralSecurityException | IllegalArgumentException error) {
                    cancelWork("서버 연결 정보를 다시 설정해 주세요.");
                }
            }

            @Override
            public void onError(String message) {
                if (guard.accepts(expected)) {
                    cancelWork(message);
                }
            }

            @Override
            public void onLevel(int percent, int seconds) {
                if (guard.accepts(expected) && recordingActivity != null && !sending) {
                    recordingActivity.progress(seconds);
                }
            }
        });
        audioWorker.execute(capture);
        refreshBubble();
    }

    private boolean unchangedTarget() {
        if (captured == null || !captured.refresh() || !eligible(captured)) {
            return false;
        }
        String current = TextInsertion.editorText(captured.getText(), captured.isShowingHintText());
        return current.equals(original)
                && TextInsertion.editorSelection(current, captured.getTextSelectionStart()) == selectionStart
                && TextInsertion.editorSelection(current, captured.getTextSelectionEnd()) == selectionEnd;
    }

    private void insert(String text) {
        AccessibilityNodeInfo current = findEditor();
        if (!unchangedTarget() || current == null || !captured.equals(current)) {
            cancelWork("입력창이나 커서가 바뀌어 결과를 입력하지 않았습니다.");
            return;
        }
        writing = true;
        boolean inserted = false;
        try {
            if (Build.VERSION.SDK_INT >= 33) {
                InputMethod method = getInputMethod();
                EditorInfo info = method == null ? null : method.getCurrentInputEditorInfo();
                InputMethod.AccessibilityInputConnection connection = method == null ? null : method.getCurrentInputConnection();
                if (connection != null && info != null && samePackage(captured, info.packageName)) {
                    connection.commitText(text, 1, null);
                    inserted = true;
                }
            }
            if (!inserted) {
                String updated = TextInsertion.replaceSelection(original, selectionStart, selectionEnd, text);
                Bundle arguments = new Bundle();
                arguments.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, updated);
                inserted = captured.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, arguments);
                if (inserted) {
                    int cursor = Math.min(selectionStart, selectionEnd) + text.length();
                    Bundle selection = new Bundle();
                    selection.putInt(AccessibilityNodeInfo.ACTION_ARGUMENT_SELECTION_START_INT, cursor);
                    selection.putInt(AccessibilityNodeInfo.ACTION_ARGUMENT_SELECTION_END_INT, cursor);
                    captured.performAction(AccessibilityNodeInfo.ACTION_SET_SELECTION, selection);
                }
            }
        } catch (IllegalArgumentException error) {
            inserted = false;
        } finally {
            writing = false;
        }
        cancelWork(inserted ? "음성 입력을 전달했습니다." : "이 앱의 입력창에서 음성 입력을 지원하지 않습니다.");
    }

    private void releaseTarget() {
        if (captured != null) {
            captured.recycle();
            captured = null;
        }
        original = null;
    }

    void cancelWork(String message) {
        boolean wasActive = guard.isActive();
        guard.invalidate();
        if (capture != null) {
            capture.cancel();
            capture = null;
        }
        if (request != null) {
            request.cancel();
            request = null;
        }
        sending = false;
        releaseTarget();
        RecordingActivity activity = recordingActivity;
        recordingActivity = null;
        if (activity != null) {
            activity.complete();
        }
        closePalette();
        refreshBubble();
        if (wasActive && message != null) {
            notice(message);
        }
        if (!destroyed) {
            scheduleUpdate();
        }
    }

    void recorderStopped(RecordingActivity activity) {
        if (recordingActivity == activity) {
            cancelWork("화면이 바뀌어 취소했습니다.");
        }
    }

    void refreshBubble() {
        String mode = preferences == null ? "minimal" : preferences.mode();
        if (bubble != null) {
            bubble.update(capture != null && !sending, sending, mode);
        }
        if (recordingActivity != null) {
            recordingActivity.update(capture != null && !sending, sending, mode);
        }
    }

    void settingsChanged() {
        refreshBubble();
        scheduleUpdate();
    }

    void showModes() {
        if (palette != null) {
            closePalette();
            return;
        }
        radialCenterX = anchorX + Ui.dp(this, 28);
        radialCenterY = anchorY + Ui.dp(this, 28);
        int width = Ui.dp(this, 176);
        int height = Ui.dp(this, 292);
        int x = Math.max(0, Math.round(radialCenterX) - Ui.dp(this, 146));
        int y = Math.max(0, Math.min(Math.round(radialCenterY) - Ui.dp(this, 146), displayHeight - height));
        palette = new RadialModeView(this, radialCenterX - x, radialCenterY - y, preferences.mode(), this::chooseMode);
        try {
            windows.addView(palette, overlayParams(width, height, x, y));
        } catch (WindowManager.BadTokenException error) {
            palette = null;
        }
    }

    private void chooseMode(int index) {
        if (index >= 0 && index < Modes.IDS.length) {
            preferences.mode(Modes.IDS[index]);
        }
        closePalette();
        refreshBubble();
    }

    private int radialPick(float screenX, float screenY) {
        float density = getResources().getDisplayMetrics().density;
        return RadialGeometry.pick((screenX - radialCenterX) / density, (screenY - radialCenterY) / density);
    }

    BubbleView.Gestures gestures() {
        return new BubbleView.Gestures() {
            @Override
            public void tap() {
                toggle();
            }

            @Override
            public void hold() {
                showModes();
            }

            @Override
            public void drag(float screenX, float screenY) {
                if (palette != null) {
                    palette.highlight(radialPick(screenX, screenY));
                }
            }

            @Override
            public void release(float screenX, float screenY) {
                if (palette != null) {
                    chooseMode(radialPick(screenX, screenY));
                }
            }

            @Override
            public void cancel() {
                closePalette();
            }
        };
    }

    private void closePalette() {
        if (palette != null) {
            windows.removeView(palette);
            palette = null;
        }
    }

    private void notice(String message) {
        Toast.makeText(this, message, Toast.LENGTH_SHORT).show();
    }

    @Override
    public void onInterrupt() {
        cancelWork("음성 입력을 중단했습니다.");
    }

    @Override
    public void onDestroy() {
        destroyed = true;
        instance.clear();
        main.removeCallbacksAndMessages(null);
        cancelWork(null);
        hideBubble();
        closePalette();
        audioWorker.shutdownNow();
        networkWorker.shutdownNow();
        super.onDestroy();
    }
}
