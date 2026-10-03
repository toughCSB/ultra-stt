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
            android.accessibilityservice.AccessibilityServiceInfo automationInfo = automation.getServiceInfo();
            automationInfo.flags |= android.accessibilityservice.AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS;
            automation.setServiceInfo(automationInfo);
            SystemClock.sleep(500);
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
            if (!"current".equals(arguments.getString("scenario"))) {
                Intent editor = new Intent(getContext(), EditorActivity.class);
                editor.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
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
            require(waitForRecording(true, 4000), "Recording cancelled instead of starting");
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
