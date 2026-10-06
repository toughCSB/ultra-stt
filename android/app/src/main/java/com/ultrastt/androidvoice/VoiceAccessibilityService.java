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
import android.view.inputmethod.SurroundingText;
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
    private boolean updatePending;
    private final Runnable update = () -> {
        updatePending = false;
        updateTarget();
    };
    private Preferences preferences;
    private WindowManager windows;
    private BubbleView bubble;
    private RadialModeView palette;
    private AccessibilityNodeInfo focused;
    private AccessibilityNodeInfo captured;
    private EditorInfo capturedEditorInfo;
    private InputMethod.AccessibilityInputConnection capturedConnection;
    private String original;
    private int selectionStart;
    private int selectionEnd;
    private AudioCapture capture;
    private TranscriptionRequest request;
    private RecordingActivity recordingActivity;
    private boolean sending;
    private boolean writing;
    private String targetLostReason;
    private boolean connectionTarget;
    private boolean keyboardSuppressed;
    private int previousKeyboardMode;
    private long keyboardHideDeadline;
    private boolean preparingKeyboard;
    private boolean launchingRecorder;
    private boolean destroyed;
    private long session;
    private int anchorX;
    private int anchorY;
    private final Rect safeDisplay = new Rect();
    private final Rect dragBounds = new Rect();
    private boolean movingAnchor;
    private int moveOriginX;
    private int moveOriginY;
    private boolean radialRight;
    private int radialSelection = -1;
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
                if (guard.isActive() && !writing && !preparingKeyboard && captured != null && info != capturedEditorInfo) {
                    targetChanged("입력창이 바뀌었습니다.");
                }
                scheduleUpdate();
            }

            @Override
            public void onFinishInput() {
                if (guard.isActive() && !writing && !preparingKeyboard) {
                    targetChanged("입력창이 닫혔습니다.");
                }
                super.onFinishInput();
                scheduleUpdate();
            }

            @Override
            public void onUpdateSelection(int oldStart, int oldEnd, int newStart, int newEnd,
                    int candidatesStart, int candidatesEnd) {
                super.onUpdateSelection(oldStart, oldEnd, newStart, newEnd, candidatesStart, candidatesEnd);
                if (guard.isActive() && !writing && original != null
                        && (TextInsertion.editorSelection(original, newStart) != selectionStart
                        || TextInsertion.editorSelection(original, newEnd) != selectionEnd)) {
                    targetChanged("커서가 이동했습니다.");
                }
            }
        };
    }

    @Override
    public void onAccessibilityEvent(AccessibilityEvent event) {
        if (destroyed || preferences == null) {
            return;
        }
        if (preparingKeyboard && event.getEventType() == AccessibilityEvent.TYPE_VIEW_CLICKED) {
            AccessibilityNodeInfo source = event.getSource();
            if (source != null && !source.equals(captured) && !samePackage(source, getPackageName())) {
                targetChanged("입력 대상이 바뀌었습니다.");
            }
        }
        if (guard.isActive() && !writing && !preparingKeyboard && captured != null
                && (event.getEventType() == AccessibilityEvent.TYPE_VIEW_TEXT_CHANGED
                || event.getEventType() == AccessibilityEvent.TYPE_VIEW_TEXT_SELECTION_CHANGED)) {
            AccessibilityNodeInfo source = event.getSource();
            if (source != null && captured.equals(source) && !unchangedTarget()) {
                targetChanged("입력 내용이나 커서가 바뀌었습니다.");
            }
        }
        scheduleUpdate();
    }

    private void scheduleUpdate() {
        if (!destroyed && !updatePending) {
            updatePending = true;
            main.postDelayed(update, 100);
        }
    }

    private boolean samePackage(AccessibilityNodeInfo node, String packageName) {
        return node.getPackageName() != null && node.getPackageName().toString().equals(packageName);
    }

    private boolean eligible(AccessibilityNodeInfo node) {
        if (node == null || !node.isEditable() || !focusedTarget(node) || !node.isVisibleToUser()
                || !node.isEnabled() || node.isPassword()) {
            return false;
        }
        if (samePackage(node, getPackageName())) {
            return (getPackageName() + ":id/voice_test_editor").equals(node.getViewIdResourceName());
        }
        return true;
    }

    private boolean focusedTarget(AccessibilityNodeInfo node) {
        if (node.isFocused()) {
            return true;
        }
        if (Build.VERSION.SDK_INT < 33 || !guard.isActive() || captured == null || !captured.equals(node)
                || recordingActivity == null || !recordingActivity.isRecordingForeground()) {
            return false;
        }
        return sameInputConnection();
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
        if (node != null && node.refresh() && eligible(node)) {
            return node;
        }
        for (AccessibilityWindowInfo window : visibleWindows) {
            if (window.getType() != AccessibilityWindowInfo.TYPE_APPLICATION) {
                continue;
            }
            AccessibilityNodeInfo root = window.getRoot();
            if (root != null) {
                node = root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT);
                if (node != null && node.refresh() && eligible(node)) {
                    return node;
                }
                if (window.isActive() || window.isFocused()) {
                    node = findFocusedDescendant(root, new int[]{256}, 0);
                    if (node != null) {
                        return node;
                    }
                }
            }
        }
        return eligible(captured) ? captured : null;
    }

    private AccessibilityNodeInfo findFocusedDescendant(AccessibilityNodeInfo node, int[] remaining, int depth) {
        if (remaining[0]-- <= 0 || depth > 32) {
            return null;
        }
        if (node.isEditable() && node.isFocused() && node.refresh() && eligible(node)) {
            return node;
        }
        for (int index = 0; index < node.getChildCount() && remaining[0] > 0; index++) {
            AccessibilityNodeInfo child = node.getChild(index);
            if (child == null) {
                continue;
            }
            AccessibilityNodeInfo found = findFocusedDescendant(child, remaining, depth + 1);
            if (found != child) {
                child.recycle();
            }
            if (found != null) {
                return found;
            }
        }
        return null;
    }

    private void updateTarget() {
        if (destroyed || preferences == null || preparingKeyboard || launchingRecorder) {
            return;
        }
        AccessibilityNodeInfo editor = preferences.floatingEnabled() ? findEditor() : null;
        if (guard.isActive() && targetLostReason == null
                && (editor == null || captured == null || !captured.equals(editor) || !unchangedTarget())) {
            targetChanged("입력창이 바뀌었습니다.");
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
        if ((bubble != null && bubble.isInteracting()) || palette != null) {
            return;
        }
        Rect display;
        if (Build.VERSION.SDK_INT >= 30) {
            android.view.WindowMetrics metrics = windows.getCurrentWindowMetrics();
            display = new Rect(metrics.getBounds());
            android.graphics.Insets insets = metrics.getWindowInsets().getInsetsIgnoringVisibility(
                    android.view.WindowInsets.Type.systemBars() | android.view.WindowInsets.Type.displayCutout());
            display.left += insets.left;
            display.top += insets.top;
            display.right -= insets.right;
            display.bottom -= insets.bottom;
        } else {
            android.util.DisplayMetrics metrics = new android.util.DisplayMetrics();
            windows.getDefaultDisplay().getRealMetrics(metrics);
            display = new Rect(0, Ui.dp(this, 32), metrics.widthPixels, metrics.heightPixels - Ui.dp(this, 32));
        }
        int margin = Ui.dp(this, 8);
        int size = Ui.dp(this, BubblePlacement.TOUCH_SIZE);
        safeDisplay.set(display);
        safeDisplay.inset(margin, margin);
        Rect available = new Rect(safeDisplay);
        Rect field = new Rect();
        editor.getBoundsInScreen(field);
        int keyboardTop = safeDisplay.bottom;
        boolean keyboardVisible = false;
        for (AccessibilityWindowInfo window : getWindows()) {
            if (window.getId() == editor.getWindowId()) {
                Rect bounds = new Rect();
                window.getBoundsInScreen(bounds);
                available.intersect(bounds);
            }
            if (window.getType() == AccessibilityWindowInfo.TYPE_INPUT_METHOD) {
                Rect bounds = new Rect();
                window.getBoundsInScreen(bounds);
                if (!bounds.isEmpty() && bounds.bottom > field.top) {
                    keyboardTop = Math.min(keyboardTop, bounds.top);
                    keyboardVisible = true;
                }
            }
        }
        float density = getResources().getDisplayMetrics().density;
        int safeBottom = Math.round(BubblePlacement.highestAllowedBottom(field.top / density,
                available.bottom / density, keyboardTop / density) * density);
        dragBounds.set(available.left, available.top,
                Math.max(available.left, available.right - size),
                Math.max(available.top, safeBottom - size));
        int desired = Math.round(BubblePlacement.defaultTop(field.top / density, field.bottom / density,
                keyboardTop / density, keyboardVisible) * density);
        float savedX = preferences.bubbleX();
        float savedY = preferences.bubbleY();
        if (Float.isFinite(savedX) && Float.isFinite(savedY)) {
            anchorX = Math.round(BubblePlacement.restore(savedX, safeDisplay.left, safeDisplay.right - size));
            anchorY = Math.round(BubblePlacement.restore(savedY, safeDisplay.top, safeDisplay.bottom - size));
        } else {
            anchorX = dragBounds.right;
            anchorY = desired;
        }
        anchorX = Math.round(BubblePlacement.clamp(anchorX, dragBounds.left, dragBounds.right));
        anchorY = Math.round(BubblePlacement.clamp(anchorY, dragBounds.top, dragBounds.bottom));
        updateBubblePosition();
    }

    private void updateBubblePosition() {
        if (bubble != null) {
            int size = Ui.dp(this, BubblePlacement.TOUCH_SIZE);
            windows.updateViewLayout(bubble, overlayParams(size, size, anchorX, anchorY));
        }
    }

    private void moveBubble(float deltaX, float deltaY) {
        if (!movingAnchor) {
            movingAnchor = true;
            moveOriginX = anchorX;
            moveOriginY = anchorY;
        }
        anchorX = Math.round(BubblePlacement.clamp(moveOriginX + deltaX, dragBounds.left, dragBounds.right));
        anchorY = Math.round(BubblePlacement.clamp(moveOriginY + deltaY, dragBounds.top, dragBounds.bottom));
        updateBubblePosition();
    }

    private void finishMoving(boolean cancelled) {
        if (!movingAnchor) {
            return;
        }
        movingAnchor = false;
        if (cancelled) {
            anchorX = moveOriginX;
            anchorY = moveOriginY;
        } else {
            int size = Ui.dp(this, BubblePlacement.TOUCH_SIZE);
            preferences.bubblePosition(
                    BubblePlacement.fraction(anchorX, safeDisplay.left, safeDisplay.right - size),
                    BubblePlacement.fraction(anchorY, safeDisplay.top, safeDisplay.bottom - size));
        }
        updateBubblePosition();
        scheduleUpdate();
    }

    @Override
    public void onConfigurationChanged(android.content.res.Configuration configuration) {
        super.onConfigurationChanged(configuration);
        if (bubble != null) {
            bubble.cancelGesture();
        }
        closePalette();
        scheduleUpdate();
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
            int size = Ui.dp(this, BubblePlacement.TOUCH_SIZE);
            windows.addView(bubble, overlayParams(size, size, anchorX, anchorY));
            refreshBubble();
        } catch (WindowManager.BadTokenException error) {
            bubble = null;
        }
    }

    private void hideBubble() {
        if (bubble != null) {
            BubbleView previous = bubble;
            previous.cancelGesture();
            bubble = null;
            windows.removeView(previous);
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
        if (Build.VERSION.SDK_INT >= 33) {
            InputMethod method = getInputMethod();
            EditorInfo info = method == null ? null : method.getCurrentInputEditorInfo();
            if (info != null && samePackage(captured, info.packageName)) {
                capturedEditorInfo = info;
                capturedConnection = method.getCurrentInputConnection();
            }
        }
        original = TextInsertion.editorText(editor.getText(), editor.isShowingHintText());
        selectionStart = TextInsertion.editorSelection(original, editor.getTextSelectionStart());
        selectionEnd = TextInsertion.editorSelection(original, editor.getTextSelectionEnd());
        if (Build.VERSION.SDK_INT >= 33 && (selectionStart < 0 || selectionEnd < 0)) {
            SurroundingText actual = connectedText();
            if (actual != null && actual.getOffset() == 0) {
                original = actual.getText().toString();
                selectionStart = actual.getSelectionStart();
                selectionEnd = actual.getSelectionEnd();
                connectionTarget = true;
            }
        }
        if (selectionStart < 0 || selectionEnd < 0 || selectionStart > original.length()
                || selectionEnd > original.length() || original.length() > 200000) {
            releaseTarget();
            notice("이 입력창의 커서를 안전하게 확인할 수 없어 녹음하지 않았습니다.");
            return;
        }
        session = guard.begin();
        targetLostReason = null;
        hideBubble();
        preparingKeyboard = true;
        keyboardHideDeadline = android.os.SystemClock.uptimeMillis() + 1500;
        hideKeyboardForRecording();
        long expected = session;
        main.postDelayed(() -> prepareHiddenEditor(expected, false), 250);
    }

    private void launchRecorder() {
        preparingKeyboard = false;
        launchingRecorder = true;
        keyboardHideDeadline = 0;
        Intent recorder = new Intent(this, RecordingActivity.class);
        recorder.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_NO_ANIMATION);
        recorder.putExtra("session", session);
        try {
            startActivity(recorder);
            long expected = session;
            main.postDelayed(() -> {
                if (guard.accepts(expected) && launchingRecorder) {
                    cancelWork("녹음 창을 열지 못했습니다. 앱을 한 번 열고 다시 시도해 주세요.");
                }
            }, 2500);
        } catch (RuntimeException error) {
            cancelWork("녹음 창을 열지 못했습니다. 접근성 설정을 확인해 주세요.");
        }
    }

    boolean attachRecorder(RecordingActivity activity, long expected) {
        if (!guard.accepts(expected) || captured == null) {
            return false;
        }
        recordingActivity = activity;
        return true;
    }

    void beginCapture(long expected) {
        if (!guard.accepts(expected) || capture != null || sending) {
            return;
        }
        launchingRecorder = false;
        if (!unchangedTarget()) {
            cancelWork("입력창이 바뀌어 취소했습니다.");
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
                                insert(expected, text);
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

    private void hideKeyboardForRecording() {
        SoftKeyboardController controller = getSoftKeyboardController();
        previousKeyboardMode = controller.getShowMode();
        if (previousKeyboardMode != SHOW_MODE_HIDDEN) {
            keyboardSuppressed = controller.setShowMode(SHOW_MODE_HIDDEN);
        }
    }

    private void prepareHiddenEditor(long expected, boolean focusRequested) {
        if (!guard.accepts(expected) || !preparingKeyboard) {
            return;
        }
        AccessibilityNodeInfo current = findEditor();
        AccessibilityNodeInfo active = getRootInActiveWindow();
        PowerManager power = (PowerManager) getSystemService(POWER_SERVICE);
        KeyguardManager keyguard = (KeyguardManager) getSystemService(KEYGUARD_SERVICE);
        if (!power.isInteractive() || keyguard.isKeyguardLocked()
                || captured == null || !captured.refresh() || !captured.isEditable()
                || !captured.isEnabled() || captured.isPassword() || (current != null && !captured.equals(current))
                || active == null || !samePackage(active, captured.getPackageName().toString())
                || active.getWindowId() != captured.getWindowId()
                || android.os.SystemClock.uptimeMillis() >= keyboardHideDeadline) {
            cancelWork("입력창이 닫혀 취소했습니다.");
            return;
        }
        if (!captured.isVisibleToUser()) {
            boolean requested = focusRequested;
            main.postDelayed(() -> prepareHiddenEditor(expected, requested), 50);
            return;
        }
        for (AccessibilityWindowInfo window : getWindows()) {
            if (window.getType() == AccessibilityWindowInfo.TYPE_INPUT_METHOD && keyboardSuppressed) {
                boolean requested = focusRequested;
                main.postDelayed(() -> prepareHiddenEditor(expected, requested), 50);
                return;
            }
        }
        if (unchangedTarget()) {
            position(captured);
            launchRecorder();
            return;
        }
        String currentText = TextInsertion.editorText(captured.getText(), captured.isShowingHintText());
        if (!original.equals(currentText)
                || TextInsertion.editorSelection(currentText, captured.getTextSelectionStart()) != selectionStart
                || TextInsertion.editorSelection(currentText, captured.getTextSelectionEnd()) != selectionEnd) {
            cancelWork("입력 내용이나 커서가 바뀌어 취소했습니다.");
            return;
        }
        if (!captured.isFocused() && !focusRequested) {
            if (!captured.performAction(AccessibilityNodeInfo.ACTION_FOCUS)) {
                cancelWork("입력창의 커서를 유지하지 못해 취소했습니다.");
                return;
            }
            focusRequested = true;
        }
        if (Build.VERSION.SDK_INT < 33) {
            cancelWork("입력창의 커서를 유지하지 못해 취소했습니다.");
            return;
        }
        InputMethod method = getInputMethod();
        EditorInfo info = method == null ? null : method.getCurrentInputEditorInfo();
        InputMethod.AccessibilityInputConnection connection = method == null ? null : method.getCurrentInputConnection();
        if (captured.refresh() && captured.isFocused() && info != null && connection != null
                && samePackage(captured, info.packageName)) {
            SurroundingText actual = connection.getSurroundingText(200001, 200001, 0);
            if (!matchesCapturedText(actual)) {
                cancelWork("입력 내용이나 커서가 바뀌어 취소했습니다.");
                return;
            }
            capturedEditorInfo = info;
            capturedConnection = connection;
            connectionTarget = true;
            position(captured);
            launchRecorder();
            return;
        }
        boolean requested = focusRequested;
        main.postDelayed(() -> prepareHiddenEditor(expected, requested), 50);
    }

    private void restoreKeyboardMode() {
        if (keyboardSuppressed) {
            keyboardSuppressed = false;
            SoftKeyboardController controller = getSoftKeyboardController();
            if (controller.getShowMode() == SHOW_MODE_HIDDEN) {
                controller.setShowMode(previousKeyboardMode);
            }
        }
    }

    private boolean unchangedTarget() {
        if (captured == null || !captured.refresh() || !eligible(captured)) {
            return false;
        }
        if (capturedConnection != null && !sameInputConnection()) {
            return false;
        }
        if (connectionTarget && Build.VERSION.SDK_INT >= 33) {
            SurroundingText actual = connectedText();
            return matchesCapturedText(actual);
        }
        String current = TextInsertion.editorText(captured.getText(), captured.isShowingHintText());
        return current.equals(original)
                && TextInsertion.editorSelection(current, captured.getTextSelectionStart()) == selectionStart
                && TextInsertion.editorSelection(current, captured.getTextSelectionEnd()) == selectionEnd;
    }

    private SurroundingText connectedText() {
        if (Build.VERSION.SDK_INT < 33 || !sameInputConnection()) {
            return null;
        }
        return capturedConnection.getSurroundingText(200001, 200001, 0);
    }

    @android.annotation.TargetApi(31)
    private boolean matchesCapturedText(SurroundingText actual) {
        if (actual == null || !original.contentEquals(actual.getText())
                || actual.getSelectionStart() != selectionStart || actual.getSelectionEnd() != selectionEnd) {
            return false;
        }
        return actual.getOffset() == 0 || (actual.getOffset() == -1 && original.isEmpty()
                && selectionStart == 0 && selectionEnd == 0 && !captured.isShowingHintText()
                && captured.getText() != null && captured.getText().length() == 0);
    }

    private boolean sameInputConnection() {
        if (Build.VERSION.SDK_INT < 33 || captured == null || capturedConnection == null || capturedEditorInfo == null) {
            return false;
        }
        InputMethod method = getInputMethod();
        return method != null && method.getCurrentInputEditorInfo() == capturedEditorInfo
                && method.getCurrentInputConnection() != null && samePackage(captured, capturedEditorInfo.packageName);
    }

    private void insert(long expected, String text) {
        if (targetLostReason != null) {
            recoverResult(expected, text, targetLostReason);
            return;
        }
        try {
            AccessibilityNodeInfo current = findEditor();
            if (captured == null || !unchangedTarget() || current == null || !captured.equals(current)) {
                recoverResult(expected, text, "입력창이나 커서가 바뀌었습니다.");
                return;
            }
        } catch (RuntimeException error) {
            recoverResult(expected, text, "입력창을 확인하지 못했습니다.");
            return;
        }
        writing = true;
        boolean inserted = false;
        try {
            if (Build.VERSION.SDK_INT >= 33) {
                if (sameInputConnection()) {
                    capturedConnection.commitText(text, 1, null);
                    inserted = true;
                }
            }
            if (!inserted && !connectionTarget) {
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
        } catch (RuntimeException error) {
            inserted = false;
        } finally {
            writing = false;
        }
        if (inserted) {
            finishWork("음성 입력을 전달했습니다.", true);
        } else {
            recoverResult(expected, text, "이 앱의 입력창에서 음성 입력을 지원하지 않습니다.");
        }
    }

    private void recoverResult(long expected, String text, String message) {
        if (!guard.accepts(expected)) {
            return;
        }
        RecoveryResult.Snapshot saved = RecoveryResult.ACTIVE.saveIfCurrent(guard, expected, text);
        if (saved != null) {
            finishWork(message, false, saved);
        } else {
            finishWork("인식된 문장이 없어 입력하지 않았습니다.", false);
        }
    }

    private void targetChanged(String message) {
        if (!guard.isActive() || targetLostReason != null) {
            return;
        }
        if (!sending) {
            cancelWork(message + " 음성 입력을 취소했습니다.");
            return;
        }
        targetLostReason = message;
        releaseTarget();
    }

    private void releaseTarget() {
        if (captured != null) {
            captured.recycle();
            captured = null;
        }
        original = null;
        capturedEditorInfo = null;
        capturedConnection = null;
        connectionTarget = false;
    }

    void cancelWork(String message) {
        finishWork(message, false);
    }

    private void finishWork(String message, boolean delivered) {
        finishWork(message, delivered, null);
    }

    private void finishWork(String message, boolean delivered, RecoveryResult.Snapshot recovery) {
        boolean wasActive = guard.isActive();
        guard.invalidate();
        targetLostReason = null;
        preparingKeyboard = false;
        launchingRecorder = false;
        keyboardHideDeadline = 0;
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
        boolean availableActivity = activity != null && !activity.isFinishing() && !activity.isDestroyed();
        boolean showingResult = delivered && availableActivity;
        if (showingResult) {
            activity.delivered();
        } else if (recovery != null && availableActivity) {
            activity.recover(recovery);
        } else {
            recordingActivity = null;
            if (activity != null) {
                activity.complete();
            }
            restoreKeyboardMode();
        }
        closePalette();
        refreshBubble();
        if (wasActive && !showingResult) {
            if (recovery != null) {
                if (!availableActivity) {
                    notice("완성문을 입력하지 못했습니다. Ultra STT 앱에서 복사해 주세요.");
                }
            } else if (message != null) {
                notice(message);
            }
        }
        if (!destroyed) {
            scheduleUpdate();
        }
    }

    void recorderDismissed(RecordingActivity activity) {
        if (recordingActivity == activity) {
            recordingActivity = null;
            restoreKeyboardMode();
            scheduleUpdate();
        }
    }

    void selectRecordingMode(String mode) {
        if (guard.isActive() && !sending) {
            preferences.mode(Modes.IDS[Modes.index(mode)]);
            refreshBubble();
        }
    }

    void recorderStopped(RecordingActivity activity) {
        if (recordingActivity == activity) {
            if (sending) {
                targetChanged("화면이 바뀌었습니다.");
                activity.complete();
                recorderDismissed(activity);
            } else {
                cancelWork("화면이 바뀌어 취소했습니다.");
            }
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
        radialCenterX = anchorX + Ui.dp(this, BubblePlacement.TOUCH_SIZE / 2);
        radialCenterY = anchorY + Ui.dp(this, BubblePlacement.TOUCH_SIZE / 2);
        radialRight = radialCenterX < safeDisplay.exactCenterX();
        radialSelection = -1;
        int width = Ui.dp(this, 176);
        int height = Ui.dp(this, 292);
        int x = Math.round(BubblePlacement.clamp(radialCenterX - Ui.dp(this, radialRight ? 30 : 146),
                safeDisplay.left, safeDisplay.right - width));
        int y = Math.round(BubblePlacement.clamp(radialCenterY - Ui.dp(this, 146),
                safeDisplay.top, safeDisplay.bottom - height));
        radialCenterY = y + Ui.dp(this, 146);
        palette = new RadialModeView(this, radialCenterX - x, radialCenterY - y,
                radialRight, preferences.mode(), this::chooseMode);
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
        return RadialGeometry.pick((screenX - radialCenterX) / density, (screenY - radialCenterY) / density, radialRight);
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
                    radialSelection = radialPick(screenX, screenY);
                    palette.highlight(radialSelection);
                }
            }

            @Override
            public void release(float screenX, float screenY) {
                if (palette != null) {
                    chooseMode(radialSelection < 0 ? -1 : radialPick(screenX, screenY));
                }
            }

            @Override
            public void cancel() {
                closePalette();
            }

            @Override
            public void move(float deltaX, float deltaY) {
                moveBubble(deltaX, deltaY);
            }

            @Override
            public void finishMove(boolean cancelled) {
                finishMoving(cancelled);
            }
        };
    }

    private void closePalette() {
        if (palette != null) {
            windows.removeView(palette);
            palette = null;
            scheduleUpdate();
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
