package com.ultrastt.androidvoice;

import android.app.Activity;
import android.os.Bundle;
import android.graphics.Color;
import android.text.InputType;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.TextView;

public final class EditorActivity extends Activity {
    @Override
    public void onCreate(Bundle state) {
        super.onCreate(state);
        LinearLayout content = new LinearLayout(this);
        content.setOrientation(LinearLayout.VERTICAL);
        content.setPadding(24, 100, 24, 24);
        content.setBackgroundColor(Color.WHITE);
        TextView title = new TextView(this);
        title.setText("Ultra STT QA · 외부 앱 커서 테스트");
        title.setTextSize(20);
        title.setTextColor(Color.BLACK);
        content.addView(title);
        EditText editor = new EditText(this);
        editor.setId(android.R.id.edit);
        editor.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_MULTI_LINE);
        editor.setHint("QA 전용 빈 입력창 · 실제 대화가 아닙니다");
        editor.setTextColor(Color.BLACK);
        editor.setHintTextColor(Color.GRAY);
        content.addView(editor, new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, 240));
        setContentView(content);
        editor.requestFocus();
    }
}
