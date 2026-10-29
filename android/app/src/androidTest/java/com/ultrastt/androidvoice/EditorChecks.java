package com.ultrastt.androidvoice;

import android.app.Activity;
import android.app.Instrumentation;
import android.app.UiAutomation;
import android.content.Intent;
import android.graphics.Rect;
import android.os.Bundle;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.MotionEvent;
import android.view.accessibility.AccessibilityNodeInfo;

public final class EditorChecks extends Instrumentation {
    private Bundle arguments;
    private UiAutomation automation;
    private volatile String voiceNotice = "none";

    @Override
    public void onCreate(Bundle options) {
        super.onCreate(options);
        arguments = options == null ? new Bundle() : options;
        start();
    }

    @Override
    public void onStart() {
        Bundle result = new Bundle();
        try {
            automation = getUiAutomation(UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES);
            automation.setOnAccessibilityEventListener(event -> {
                if (event.getEventType() == android.view.accessibility.AccessibilityEvent.TYPE_NOTIFICATION_STATE_CHANGED
                        && event.getPackageName() != null && "com.ultrastt.androidvoice".contentEquals(event.getPackageName())) {
                    voiceNotice = event.getText().toString();
                }
            });
            android.accessibilityservice.AccessibilityServiceInfo automationInfo = automation.getServiceInfo();
            automationInfo.flags |= android.accessibilityservice.AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS;
            automation.setServiceInfo(automationInfo);
            SystemClock.sleep(500);
            if ("current-drag".equals(arguments.getString("scenario"))) {
                checkCurrentDrag();
                result.putString("stream", "PASS: actual search field downward drag, horizontal drag, refresh persistence, cancel and radial menu; no recording or upload\n");
                finish(Activity.RESULT_OK, result);
                return;
            }
            if ("current-observe".equals(arguments.getString("scenario"))) {
                require(waitForOverlay(3000) != null, "Voice button missing without extra editor tap");
                result.putString("stream", "PASS: button visible without extra editor tap; no recording\n");
                finish(Activity.RESULT_OK, result);
                return;
            }
            if ("panel-states".equals(arguments.getString("scenario"))) {
                runOnMainSync(this::checkPanelStates);
                result.putString("stream", "PASS: panel timer, mode selection, stop, processing lock and delivered state; no audio uploaded\n");
                finish(Activity.RESULT_OK, result);
                return;
            }
            if ("recovery-states".equals(arguments.getString("scenario"))) {
                runOnMainSync(this::checkRecoveryStates);
                result.putString("stream", "PASS: recovery full text, stale copy rejection, discard and empty state; no clipboard overwritten or audio uploaded\n");
                finish(Activity.RESULT_OK, result);
                return;
            }
            if (!"current".equals(arguments.getString("scenario"))) {
                if ("bubble-gestures".equals(arguments.getString("scenario"))) {
                    Intent settings = new Intent();
                    settings.setClassName("com.ultrastt.androidvoice", "com.ultrastt.androidvoice.SettingsActivity");
                    settings.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                    getContext().startActivity(settings);
                    SystemClock.sleep(700);
                    AccessibilityNodeInfo reset = findLabel(automation.getRootInActiveWindow(), "마이크 위치 초기화");
                    require(reset != null && reset.performAction(AccessibilityNodeInfo.ACTION_CLICK),
                            "Default-position test must reset the previously saved drag position");
                }
                Intent editor = new Intent(getContext(), EditorActivity.class);
                editor.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                editor.putExtra("bubbleGestures", "bubble-gestures".equals(arguments.getString("scenario")));
                editor.putExtra("launchEdit", "launch-edit".equals(arguments.getString("scenario")));
                editor.putExtra("busy", "busy".equals(arguments.getString("scenario")));
                editor.putExtra("staleFocus", arguments.getString("scenario", "").startsWith("stale-focus"));
                editor.putExtra("password", "stale-focus-password".equals(arguments.getString("scenario")));
                editor.putExtra("unfocused", "stale-focus-unfocused".equals(arguments.getString("scenario")));
                editor.putExtra("lateVisible", "late-visible".equals(arguments.getString("scenario")));
                editor.putExtra("placeholder", arguments.getString("scenario", "").startsWith("placeholder"));
                editor.putExtra("paused", "paused".equals(arguments.getString("scenario")));
                editor.putExtra("keyboardBlur", arguments.getString("scenario", "").startsWith("keyboard-blur"));
                editor.putExtra("content", "cursor".equals(arguments.getString("scenario"))
                        || "text".equals(arguments.getString("scenario"))
                        || "placeholder-content".equals(arguments.getString("scenario"))
                        || "keyboard-blur-content".equals(arguments.getString("scenario")));
                getContext().startActivity(editor);
                SystemClock.sleep(700);
                if ("stale-focus-password".equals(arguments.getString("scenario"))
                        || "stale-focus-unfocused".equals(arguments.getString("scenario"))) {
                    require(waitForOverlay(1800) == null, "Unsafe descendant must not display the voice button");
                    require(waitForRecording(false, 500), "Unsafe descendant must not record");
                    automation.performGlobalAction(android.accessibilityservice.AccessibilityService.GLOBAL_ACTION_HOME);
                    result.putString("stream", "PASS: password or unfocused virtual editor rejected; no audio uploaded\n");
                    finish(Activity.RESULT_OK, result);
                    return;
                }
                AccessibilityNodeInfo field = findEditable();
                require(field != null, "QA editable field missing");
                Rect bounds = new Rect();
                field.getBoundsInScreen(bounds);
                tap(bounds.centerX(), bounds.centerY());
            } else {
                AccessibilityNodeInfo field = findEditable();
                require(field != null, "External editable field missing");
                if (!field.isFocused()) {
                    Rect fieldBounds = new Rect();
                    field.getBoundsInScreen(fieldBounds);
                    tap(fieldBounds.centerX(), fieldBounds.centerY());
                }
            }
            Rect bounds = waitForOverlay(5000);
            require(bounds != null, "Voice button missing for active external editor");
            if ("bubble-gestures".equals(arguments.getString("scenario"))) {
                checkBubbleGestures(bounds);
                automation.performGlobalAction(android.accessibilityservice.AccessibilityService.GLOBAL_ACTION_HOME);
                result.putString("stream", "PASS: composer clearance, larger target, immediate drag, persistence, cancel, inward mode fan, tap recording; no audio uploaded\n");
                finish(Activity.RESULT_OK, result);
                return;
            }
            if ("late-visible".equals(arguments.getString("scenario"))) {
                automation.performGlobalAction(android.accessibilityservice.AccessibilityService.GLOBAL_ACTION_HOME);
                result.putString("stream", "PASS: button appears after editor becomes visible without new focus event\n");
                finish(Activity.RESULT_OK, result);
                return;
            }
            String initialKeyboardMode = shell("settings get secure accessibility_soft_keyboard_mode").trim();
            if (arguments.getString("scenario", "").startsWith("keyboard")) {
                require(waitForKeyboard(true, 3000), "Keyboard must be visible before recording");
            }
            tap(bounds.centerX(), bounds.centerY());
            if ("launch-edit".equals(arguments.getString("scenario"))) {
                SystemClock.sleep(1500);
                require(waitForRecording(false, 1000), "Changed editor during recorder launch must not record");
                runOnMainSync(() -> require("녹음창 전환 중 사용자가 바꾼 문장".contentEquals(
                        EditorActivity.activeEditor.get().getText()), "Launch fixture must change text and preserve it"));
                require(normalizedMode(initialKeyboardMode) == normalizedMode(shell("settings get secure accessibility_soft_keyboard_mode").trim()),
                        "Launch cancellation must restore keyboard policy");
                automation.performGlobalAction(android.accessibilityservice.AccessibilityService.GLOBAL_ACTION_HOME);
                result.putString("stream", "PASS: actual content change during recorder handoff still cancels safely; no audio uploaded\n");
                finish(Activity.RESULT_OK, result);
                return;
            }
            if ("keyboard-cancel".equals(arguments.getString("scenario"))
                    || "keyboard-blur-content".equals(arguments.getString("scenario"))
                    || "keyboard-edit".equals(arguments.getString("scenario"))) {
                if ("keyboard-cancel".equals(arguments.getString("scenario"))) {
                    automation.performGlobalAction(android.accessibilityservice.AccessibilityService.GLOBAL_ACTION_HOME);
                } else if ("keyboard-edit".equals(arguments.getString("scenario"))) {
                    runOnMainSync(() -> EditorActivity.activeEditor.get().setText("준비 중 사용자가 변경한 내용"));
                }
                SystemClock.sleep(1800);
                require(waitForRecording(false, 1000), "Unsafe preparation must not start recording");
                require(normalizedMode(initialKeyboardMode) == normalizedMode(shell("settings get secure accessibility_soft_keyboard_mode").trim()),
                        "Keyboard policy must be restored after interrupted preparation");
                if ("keyboard-blur-content".equals(arguments.getString("scenario"))) {
                    require(EditorActivity.keyboardTransition, "Fixture must lose focus when keyboard hides");
                    runOnMainSync(() -> require("앞 뒤".contentEquals(EditorActivity.activeEditor.get().getText()), "Original text must remain untouched"));
                }
                automation.performGlobalAction(android.accessibilityservice.AccessibilityService.GLOBAL_ACTION_HOME);
                result.putString("stream", "PASS: unsafe preparation cancelled; keyboard policy restored; no audio uploaded\n");
                finish(Activity.RESULT_OK, result);
                return;
            }
            require(waitForRecording(true, 4000), "Recording cancelled instead of starting; app notice: " + voiceNotice);
            require(waitForKeyboard(false, 3000), "Keyboard must hide while recording");
            SystemClock.sleep(700);
            require(waitForRecording(true, 500), "Recording cancelled after external Activity transition");
            if ("panel-preview".equals(arguments.getString("scenario"))) {
                SystemClock.sleep(20000);
            }
            Rect panelBounds = recordingPanelBounds();
            require(panelBounds != null, "Central recording window must be visible");
            android.util.DisplayMetrics display = getContext().getResources().getDisplayMetrics();
            require(Math.abs(panelBounds.centerX() - display.widthPixels / 2) < 40 * display.density
                            && Math.abs(panelBounds.centerY() - display.heightPixels / 2) < 40 * display.density,
                    "Recording panel must be centered, not anchored beside input field");
            if ("panel-cancel".equals(arguments.getString("scenario"))) {
                tap(panelBounds.right - Math.round(44 * display.density),
                        panelBounds.top + Math.round(36 * display.density));
                require(waitForRecording(false, 2000), "Panel cancel must stop the microphone");
            }
            if ("keyboard-blur".equals(arguments.getString("scenario"))) {
                require(EditorActivity.keyboardTransition, "Fixture must lose focus when keyboard hides");
            }
            if ("cursor".equals(arguments.getString("scenario")) || "text".equals(arguments.getString("scenario"))
                    || "restart".equals(arguments.getString("scenario"))) {
                runOnMainSync(() -> {
                    android.widget.EditText field = EditorActivity.activeEditor.get();
                    require(field != null, "QA editor no longer available");
                    if ("cursor".equals(arguments.getString("scenario"))) {
                        field.setSelection(0);
                    } else if ("text".equals(arguments.getString("scenario"))) {
                        field.setText("사용자가 변경한 내용");
                    } else {
                        android.view.inputmethod.InputMethodManager keyboard = (android.view.inputmethod.InputMethodManager)
                                getContext().getSystemService(android.content.Context.INPUT_METHOD_SERVICE);
                        keyboard.restartInput(field);
                    }
                });
                require(waitForRecording(false, 2000), "Changed content or cursor must cancel recording");
            }
            automation.performGlobalAction(android.accessibilityservice.AccessibilityService.GLOBAL_ACTION_HOME);
            require(waitForRecording(false, 2000), "Recording did not cancel on Home");
            String restoredKeyboardMode = shell("settings get secure accessibility_soft_keyboard_mode").trim();
            require(normalizedMode(initialKeyboardMode) == normalizedMode(restoredKeyboardMode), "Keyboard policy must be restored after cancellation");
            result.putString("stream", "PASS: external editor recording; keyboard hidden and policy restored; Home cancellation; no audio uploaded\n");
            finish(Activity.RESULT_OK, result);
        } catch (Exception | AssertionError error) {
            if (automation != null) {
                automation.performGlobalAction(android.accessibilityservice.AccessibilityService.GLOBAL_ACTION_HOME);
            }
            result.putString("stream", "FAIL: " + error.getClass().getSimpleName() + ": " + error.getMessage() + "\n");
            finish(Activity.RESULT_CANCELED, result);
        }
    }

    private String shell(String command) {
        try (android.os.ParcelFileDescriptor descriptor = automation.executeShellCommand(command);
                java.io.InputStream input = new android.os.ParcelFileDescriptor.AutoCloseInputStream(descriptor);
                java.io.ByteArrayOutputStream output = new java.io.ByteArrayOutputStream()) {
            byte[] buffer = new byte[4096];
            int count;
            while ((count = input.read(buffer)) != -1 && output.size() < 4000000) {
                output.write(buffer, 0, count);
            }
            return output.toString("UTF-8");
        } catch (java.io.IOException error) {
            throw new IllegalStateException("QA shell observation failed", error);
        }
    }

    private void checkCurrentDrag() {
        Rect original = waitForOverlay(5000);
        require(original != null, "Microphone missing on current search field");
        float density = getContext().getResources().getDisplayMetrics().density;
        android.util.DisplayMetrics metrics = getContext().getResources().getDisplayMetrics();
        float destinationX = metrics.widthPixels * 0.30f;
        float destinationY = Math.min(original.centerY() + 140 * density, metrics.heightPixels - 64 * density);
        require(destinationY > original.centerY() + 40 * density, "Keep a top search field focused for this regression");
        try {
            drag(original.centerX(), original.centerY(), destinationX, destinationY, 0, false);
            Rect moved = waitForOverlay(2000);
            require(moved != null && Math.abs(moved.centerY() - destinationY) < 10 * density,
                    "Search field must not clamp downward drag: " + original + " -> " + moved);
            require(Math.abs(moved.centerX() - destinationX) < 10 * density, "Horizontal drag must follow finger");
            require(waitForRecording(false, 300), "Dragging cannot start microphone");
            SystemClock.sleep(500);
            require(moved.equals(waitForOverlay(1500)), "Refresh must retain dragged position");
            drag(moved.centerX(), moved.centerY(), moved.centerX() + 60 * density, moved.centerY(), 0, true);
            require(moved.equals(waitForOverlay(1500)), "Cancel must restore drag origin");
            long time = SystemClock.uptimeMillis();
            inject(time, MotionEvent.ACTION_DOWN, moved.centerX(), moved.centerY());
            SystemClock.sleep(android.view.ViewConfiguration.getLongPressTimeout() + 180);
            Rect menu = waitForOverlay(1500);
            inject(time, MotionEvent.ACTION_CANCEL, moved.centerX(), moved.centerY());
            require(menu != null && menu.width() > 100 * density, "Long press must open radial mode menu");
            require(moved.equals(waitForOverlay(1500)), "Mode cancellation must not change position");
            require(waitForRecording(false, 300), "Mode menu must not start microphone");
            Bundle detail = new Bundle();
            detail.putString("stream", "Verified movement " + original + " -> " + moved + "\n");
            sendStatus(0, detail);
        } finally {
            Rect current = waitForOverlay(1500);
            if (current != null) {
                drag(current.centerX(), current.centerY(), original.centerX(), original.centerY(), 0, false);
            }
        }
    }

    private void checkBubbleGestures(Rect original) {
        float density = getContext().getResources().getDisplayMetrics().density;
        Rect field = new Rect();
        AccessibilityNodeInfo editor = findEditable();
        require(editor != null, "Composer missing");
        editor.getBoundsInScreen(field);
        require(Math.abs(original.width() - 64 * density) < 3, "Touch target must be 64dp");
        require(original.bottom <= field.top - 47 * density, "Microphone must clear composer row");
        drag(original.centerX(), original.centerY(), 44 * density, original.centerY() - 70 * density, 0, false);
        Rect moved = waitForOverlay(2000);
        require(moved != null && moved.centerX() < original.centerX() - 80 * density, "Immediate drag must move microphone");
        require(waitForRecording(false, 300), "Dragging must never start recording");
        runOnMainSync(() -> EditorActivity.activeEditor.get().sendAccessibilityEvent(
                android.view.accessibility.AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED));
        SystemClock.sleep(300);
        require(moved.equals(waitForOverlay(1500)), "Editor refresh must not reset dragged location");
        drag(moved.centerX(), moved.centerY(), moved.centerX() + 90 * density, moved.centerY(), 0, true);
        require(moved.equals(waitForOverlay(1500)), "Cancelled drag must restore position");
        long time = SystemClock.uptimeMillis();
        inject(time, MotionEvent.ACTION_DOWN, moved.centerX(), moved.centerY());
        SystemClock.sleep(android.view.ViewConfiguration.getLongPressTimeout() + 250);
        Rect fan = waitForOverlay(1500);
        require(fan != null && fan.width() > moved.width() * 2, "Long press must open mode menu");
        require(fan.right > moved.right + 90 * density, "Left-edge mode menu must open inward");
        inject(time, MotionEvent.ACTION_MOVE, moved.centerX() + 108 * density, moved.centerY());
        SystemClock.sleep(120);
        inject(time, MotionEvent.ACTION_UP, moved.centerX() + 108 * density, moved.centerY());
        Rect afterMode = waitForOverlay(1500);
        require(moved.equals(afterMode), "Mode selection must not move microphone");
        require(waitForRecording(false, 300), "Mode selection must not record");
        drag(moved.centerX(), moved.centerY(), original.centerX(), original.centerY(), 0, false);
        Rect restored = waitForOverlay(1500);
        require(restored != null, "Microphone must remain after moving back");
        tap(restored.centerX(), restored.centerY());
        require(waitForRecording(true, 4000), "Tap after drag must still record");
        require(waitForKeyboard(false, 3000), "Recording must still hide keyboard");
        automation.performGlobalAction(android.accessibilityservice.AccessibilityService.GLOBAL_ACTION_HOME);
        require(waitForRecording(false, 2000), "Home must cancel recording");
    }

    private void drag(float fromX, float fromY, float toX, float toY, long hold, boolean cancel) {
        long time = SystemClock.uptimeMillis();
        inject(time, MotionEvent.ACTION_DOWN, fromX, fromY);
        SystemClock.sleep(hold);
        for (int step = 1; step <= 6; step++) {
            inject(time, MotionEvent.ACTION_MOVE, fromX + (toX - fromX) * step / 6,
                    fromY + (toY - fromY) * step / 6);
            SystemClock.sleep(25);
        }
        inject(time, cancel ? MotionEvent.ACTION_CANCEL : MotionEvent.ACTION_UP, toX, toY);
    }

    private void inject(long downTime, int action, float screenX, float screenY) {
        MotionEvent event = MotionEvent.obtain(downTime, SystemClock.uptimeMillis(), action, screenX, screenY, 0);
        event.setSource(InputDevice.SOURCE_TOUCHSCREEN);
        try {
            require(automation.injectInputEvent(event, true), "Touch injection failed");
        } finally {
            event.recycle();
        }
    }

    private Rect recordingPanelBounds() {
        java.util.regex.Matcher bounds = java.util.regex.Pattern.compile(
                "com\\.ultrastt\\.androidvoice/com\\.ultrastt\\.androidvoice\\.RecordingActivity, frame=\\[Rect\\((\\d+), (\\d+) - (\\d+), (\\d+)\\)\\]")
                .matcher(shell("dumpsys window windows"));
        if (bounds.find()) {
            return new Rect(Integer.parseInt(bounds.group(1)), Integer.parseInt(bounds.group(2)),
                    Integer.parseInt(bounds.group(3)), Integer.parseInt(bounds.group(4)));
        }
        return null;
    }

    private void checkPanelStates() {
        int[] stops = {0};
        int[] cancels = {0};
        String[] selected = {"minimal"};
        RecordingPanelView panel = new RecordingPanelView(getContext(),
                getContext().getDrawable(android.R.drawable.ic_btn_speak_now),
                getContext().getDrawable(android.R.drawable.ic_btn_speak_now),
                () -> stops[0]++, () -> cancels[0]++, value -> selected[0] = value);
        panel.update(true, false, "minimal");
        panel.progress(65);
        require(findText(panel, "01:05") != null, "Panel must show elapsed minutes and seconds");
        findText(panel, "최소 모드  ▾").performClick();
        findText(panel, "전문적으로").performClick();
        require("professional".equals(selected[0]), "Mode selection must reach callback");
        findText(panel, "■  녹음 종료").performClick();
        require(stops[0] == 1, "Stop must invoke toggle exactly once");
        panel.update(false, true, "professional");
        require(findText(panel, "변환 중…") != null, "Processing must be explicit");
        require(!findText(panel, "텍스트로 다듬고 있어요").isEnabled(), "Processing cannot resubmit");
        findText(panel, "텍스트로 다듬고 있어요").performClick();
        require(stops[0] == 1, "Processing tap must not toggle capture");
        findText(panel, "✕").performClick();
        require(cancels[0] == 1, "Processing remains cancellable");
        panel.delivered();
        require(findText(panel, "전송 완료") != null, "Delivered must be explicit");
        require(findText(panel, "✕").getVisibility() == android.view.View.INVISIBLE, "Delivered cancel must disappear");
        panel.update(true, false, "original");
        require(findText(panel, "전송 완료") != null, "Late updates must not overwrite completion");
    }

    private android.widget.TextView findText(android.view.View view, String value) {
        if (view instanceof android.widget.TextView && value.contentEquals(((android.widget.TextView) view).getText())) {
            return (android.widget.TextView) view;
        }
        if (view instanceof android.view.ViewGroup) {
            android.view.ViewGroup group = (android.view.ViewGroup) view;
            for (int index = 0; index < group.getChildCount(); index++) {
                android.widget.TextView found = findText(group.getChildAt(index), value);
                if (found != null) {
                    return found;
                }
            }
        }
        return null;
    }

    private void checkRecoveryStates() {
        EditorGuard guard = new EditorGuard();
        long session = guard.begin();
        RecoveryResult.Snapshot snapshot = RecoveryResult.ACTIVE.saveIfCurrent(guard, session, "QA 복구 문장\n두 번째 줄");
        int[] dismissed = {0};
        RecoveryView view = new RecoveryView(getContext(), () -> dismissed[0]++);
        view.show(snapshot);
        require(view.getVisibility() == android.view.View.VISIBLE, "Recovery must be visible");
        require(findText(view, snapshot.text) != null, "Recovery must display complete multiline text");
        RecoveryResult.Snapshot newer = RecoveryResult.ACTIVE.saveIfCurrent(guard, session, "더 최근 QA 문장");
        findText(view, "문장 복사").performClick();
        require(findText(view, "문장이 더 이상 남아 있지 않습니다.") != null, "Old recovery cannot copy a different result");
        findText(view, "닫기 · 문장 지우기").performClick();
        require(RecoveryResult.ACTIVE.latest() == newer, "Old close cannot remove newer recovery");
        view.show(newer);
        findText(view, "닫기 · 문장 지우기").performClick();
        require(RecoveryResult.ACTIVE.latest() == null && view.getVisibility() == android.view.View.GONE,
                "Current recovery must clear and hide");
        require(dismissed[0] == 2, "Close callback must run once per tap");
        guard.invalidate();
    }

    private boolean waitForRecording(boolean expected, long timeout) {
        long end = SystemClock.uptimeMillis() + timeout;
        do {
            boolean running = shell("cmd appops get com.ultrastt.androidvoice RECORD_AUDIO").contains("(running)");
            if (running == expected) {
                return true;
            }
            SystemClock.sleep(80);
        } while (SystemClock.uptimeMillis() < end);
        return false;
    }

    private int normalizedMode(String value) {
        return "null".equals(value) ? 0 : Integer.parseInt(value);
    }

    private boolean waitForKeyboard(boolean expected, long timeout) {
        long end = SystemClock.uptimeMillis() + timeout;
        do {
            boolean visible = shell("dumpsys input_method").contains("mInputShown=true");
            if (visible == expected) {
                return true;
            }
            SystemClock.sleep(80);
        } while (SystemClock.uptimeMillis() < end);
        return false;
    }

    private Rect waitForOverlay(long timeout) {
        long end = SystemClock.uptimeMillis() + timeout;
        Rect previous = null;
        long stableSince = 0;
        do {
            java.util.regex.Matcher bounds = java.util.regex.Pattern.compile(
                    "com\\.ultrastt\\.androidvoice, frame=\\[Rect\\((\\d+), (\\d+) - (\\d+), (\\d+)\\)\\]")
                    .matcher(shell("dumpsys window windows"));
            if (bounds.find()) {
                Rect current = new Rect(Integer.parseInt(bounds.group(1)), Integer.parseInt(bounds.group(2)),
                        Integer.parseInt(bounds.group(3)), Integer.parseInt(bounds.group(4)));
                if (current.equals(previous)) {
                    if (SystemClock.uptimeMillis() - stableSince >= 350) {
                        return current;
                    }
                } else {
                    previous = current;
                    stableSince = SystemClock.uptimeMillis();
                }
            } else {
                previous = null;
            }
            SystemClock.sleep(80);
        } while (SystemClock.uptimeMillis() < end);
        return null;
    }

    private AccessibilityNodeInfo findEditable() {
        AccessibilityNodeInfo root = automation.getRootInActiveWindow();
        return root == null ? null : findEditable(root);
    }

    private AccessibilityNodeInfo findLabel(AccessibilityNodeInfo node, String label) {
        if (node == null) {
            return null;
        }
        if (node.getText() != null && label.contentEquals(node.getText())) {
            return node;
        }
        for (int index = 0; index < node.getChildCount(); index++) {
            AccessibilityNodeInfo found = findLabel(node.getChild(index), label);
            if (found != null) {
                return found;
            }
        }
        return null;
    }


    private AccessibilityNodeInfo findEditable(AccessibilityNodeInfo node) {
        if (node.isEditable() && !node.isPassword()) {
            return node;
        }
        for (int index = 0; index < node.getChildCount(); index++) {
            AccessibilityNodeInfo child = node.getChild(index);
            AccessibilityNodeInfo found = child == null ? null : findEditable(child);
            if (found != null) {
                return found;
            }
        }
        return null;
    }

    private void tap(int x, int y) {
        long time = SystemClock.uptimeMillis();
        MotionEvent down = MotionEvent.obtain(time, time, MotionEvent.ACTION_DOWN, x, y, 0);
        MotionEvent up = MotionEvent.obtain(time, time + 70, MotionEvent.ACTION_UP, x, y, 0);
        down.setSource(InputDevice.SOURCE_TOUCHSCREEN);
        up.setSource(InputDevice.SOURCE_TOUCHSCREEN);
        try {
            require(automation.injectInputEvent(down, true) && automation.injectInputEvent(up, true), "Button gesture failed");
        } finally {
            down.recycle();
            up.recycle();
        }
    }

    private void require(boolean condition, String message) {
        if (!condition) {
            throw new AssertionError(message);
        }
    }
}
