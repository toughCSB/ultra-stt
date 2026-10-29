package com.ultrastt.androidvoice;

import android.content.Context;
import android.text.InputType;
import android.view.View;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.TextView;

final class ApiKeyPanel extends LinearLayout {
    interface Store {
        boolean hasKey();
        void save(String key) throws java.security.GeneralSecurityException;
        void clear() throws java.security.GeneralSecurityException;
    }

    private final Store store;
    private final LinearLayout savedPanel;
    private final LinearLayout editPanel;
    private final EditText input;
    private final TextView error;
    private final Button cancel;
    private final Button remove;
    private boolean editing;

    ApiKeyPanel(Context context, Store store) {
        super(context);
        this.store = store;
        setOrientation(VERTICAL);
        setSaveEnabled(false);
        addView(Ui.text(context, "OpenAI API 키", 17, true));
        savedPanel = Ui.column(context, 16);
        savedPanel.setBackground(Ui.background(context, Ui.PANEL, true));
        TextView saved = Ui.text(context, "✓ 저장 완료", 20, true);
        saved.setTextColor(Ui.GREEN);
        saved.setAccessibilityLiveRegion(View.ACCESSIBILITY_LIVE_REGION_POLITE);
        savedPanel.addView(saved);
        savedPanel.addView(Ui.text(context, "••••••••\n이 기기에 안전하게 저장된 키를 계속 사용합니다. 다시 입력하지 않아도 됩니다.", 14, false));
        Button modify = Ui.button(context, "API 키 수정");
        savedPanel.addView(modify, Ui.block(context));
        addView(savedPanel, Ui.block(context));
        editPanel = Ui.column(context, 0);
        input = new EditText(context);
        input.setHint("OpenAI API 키 입력");
        input.setContentDescription("OpenAI API 키 입력");
        input.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD);
        input.setSingleLine(true);
        input.setTextColor(Ui.TEXT);
        input.setHintTextColor(Ui.MUTED);
        input.setSaveEnabled(false);
        input.setImportantForAutofill(View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS);
        editPanel.addView(input, Ui.block(context));
        error = Ui.text(context, "", 14, false);
        Button save = Ui.button(context, "API 키 저장");
        save.setOnClickListener(view -> save());
        editPanel.addView(save, Ui.block(context));
        cancel = Ui.button(context, "수정 취소");
        cancel.setOnClickListener(view -> {
            editing = false;
            input.setText("");
            error.setText("");
            refresh();
        });
        editPanel.addView(cancel, Ui.block(context));
        remove = Ui.button(context, "저장된 API 키 삭제");
        remove.setOnClickListener(view -> new android.app.AlertDialog.Builder(context)
                .setTitle("저장된 API 키를 삭제할까요?")
                .setMessage("삭제하면 다시 키를 저장할 때까지 OpenAI 직접 연결을 사용할 수 없습니다. 기존 엔진 연결은 유지됩니다.")
                .setNegativeButton("취소", null)
                .setPositiveButton("삭제", (dialog, which) -> clear())
                .show());
        editPanel.addView(remove, Ui.block(context));
        error.setTextColor(android.graphics.Color.rgb(255, 139, 139));
        error.setAccessibilityLiveRegion(View.ACCESSIBILITY_LIVE_REGION_POLITE);
        editPanel.addView(error);
        addView(editPanel);
        addView(Ui.text(context, "저장은 이 기기에 키를 보관했다는 뜻입니다. API 연결·잔액은 자동 검사하지 않으며 저장만으로 요금이 발생하지 않습니다.", 12, false));
        modify.setOnClickListener(view -> {
            editing = true;
            input.setText("");
            error.setText("");
            refresh();
            input.requestFocus();
        });
        refresh();
    }

    void refresh() {
        boolean stored = store.hasKey();
        savedPanel.setVisibility(stored && !editing ? VISIBLE : GONE);
        editPanel.setVisibility(stored && !editing ? GONE : VISIBLE);
        cancel.setVisibility(stored ? VISIBLE : GONE);
        remove.setVisibility(stored ? VISIBLE : GONE);
    }

    private void save() {
        String value = input.getText().toString().trim();
        if (value.isEmpty()) {
            error.setText("API 키를 입력해 주세요. 기존에 저장한 키는 변경하지 않았습니다.");
            return;
        }
        try {
            store.save(value);
            input.setText("");
            error.setText("");
            editing = false;
            refresh();
            android.view.inputmethod.InputMethodManager keyboard =
                    (android.view.inputmethod.InputMethodManager) getContext().getSystemService(Context.INPUT_METHOD_SERVICE);
            if (keyboard != null) keyboard.hideSoftInputFromWindow(input.getWindowToken(), 0);
            announceForAccessibility("API 키 저장 완료. 다음 음성 입력부터 계속 사용합니다.");
        } catch (java.security.GeneralSecurityException | IllegalArgumentException failure) {
            error.setText("API 키를 저장하지 못했습니다. 이전 키는 유지됩니다. 다시 시도해 주세요.");
        }
    }

    private void clear() {
        try {
            store.clear();
            input.setText("");
            editing = false;
            error.setText("");
            refresh();
        } catch (java.security.GeneralSecurityException failure) {
            error.setText("API 키를 삭제하지 못했습니다. 다시 시도해 주세요.");
        }
    }
}
