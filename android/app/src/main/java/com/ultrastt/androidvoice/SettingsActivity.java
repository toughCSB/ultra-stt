package com.ultrastt.androidvoice;

import android.Manifest;
import android.app.Activity;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.Bundle;
import android.provider.Settings;
import android.text.InputType;
import android.view.View;
import android.view.WindowManager;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.EditText;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Spinner;
import android.widget.Switch;
import android.widget.TextView;

public final class SettingsActivity extends Activity implements android.content.SharedPreferences.OnSharedPreferenceChangeListener {
    private Preferences preferences;
    private EditText server;
    private EditText token;
    private TextView status;
    private TextView connection;
    private Spinner modeSelector;
    private RecoveryView recovery;
    private boolean probing;

    @Override
    public void onCreate(Bundle state) {
        super.onCreate(state);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_SECURE);
        preferences = new Preferences(this);
        ScrollView scroll = new ScrollView(this);
        LinearLayout content = Ui.column(this, 22);
        content.setOnApplyWindowInsetsListener((view, insets) -> {
            int padding = Ui.dp(this, 22);
            if (android.os.Build.VERSION.SDK_INT >= 30) {
                android.graphics.Insets bars = insets.getInsets(android.view.WindowInsets.Type.systemBars());
                view.setPadding(padding + bars.left, padding + bars.top, padding + bars.right, padding + bars.bottom);
            } else {
                view.setPadding(padding + insets.getSystemWindowInsetLeft(), padding + insets.getSystemWindowInsetTop(),
                        padding + insets.getSystemWindowInsetRight(), padding + insets.getSystemWindowInsetBottom());
            }
            return insets;
        });
        scroll.addView(content);
        ImageView icon = new ImageView(this);
        icon.setImageResource(R.drawable.app_icon);
        icon.setContentDescription("Ultra STT 녹색 마이크");
        content.addView(icon, new LinearLayout.LayoutParams(Ui.dp(this, 64), Ui.dp(this, 64)));
        content.addView(Ui.text(this, "Ultra STT", 30, true));
        content.addView(Ui.text(this, "키보드는 그대로. 말하기만 더 가볍게.", 17, false));
        recovery = new RecoveryView(this, null);
        content.addView(recovery, Ui.block(this));
        content.addView(Ui.text(this,
                "입력창을 누르면 작은 마이크가 나타납니다. 터치로 녹음 시작·종료. 길게 누른 채 반원 메뉴의 아이콘으로 드래그하고 손을 떼면 모드가 바뀝니다. 버튼 전체 색상으로 현재 모드를 구분합니다.", 14, false));
        connection = Ui.text(this, "", 15, true);
        content.addView(connection);
        status = Ui.text(this, "", 13, false);
        content.addView(status);
        addButton(content, "PC 서버 연결 확인", this::probeConnection);

        content.addView(Ui.text(this, "처음 한 번만 준비해 주세요", 19, true));
        addButton(content, "1  마이크 권한 허용", () -> requestPermissions(new String[]{Manifest.permission.RECORD_AUDIO}, 1));
        addButton(content, "2  작은 음성 버튼 켜기 (접근성)", this::openAccessibility);
        content.addView(Ui.text(this,
                "접근성에서 ‘Ultra STT 작은 음성 버튼’을 켜 주세요. 입력창을 감지하고 전사문을 커서에 넣는 데만 사용합니다. 화면 이력·비밀번호는 수집하지 않습니다. 기본 키보드는 바꾸지 않습니다.", 13, false));
        content.addView(Ui.text(this,
                "‘제한된 설정’이 표시되면: 휴대폰 설정 → 애플리케이션 → Ultra STT → 우측 상단 메뉴 → 제한된 설정 허용 후 접근성을 켜 주세요.", 12, false));
        Switch floating = new Switch(this);
        floating.setText(R.string.show_floating_button);
        floating.setTextColor(Ui.TEXT);
        floating.setChecked(preferences.floatingEnabled());
        floating.setOnCheckedChangeListener((button, checked) -> {
            preferences.floatingEnabled(checked);
            refreshService();
        });
        content.addView(floating, Ui.block(this));

        content.addView(Ui.text(this, "다듬기 모드", 19, true));
        Spinner modes = new Spinner(this);
        modeSelector = modes;
        ArrayAdapter<String> adapter = new ArrayAdapter<>(this, android.R.layout.simple_spinner_item, Modes.LABELS);
        adapter.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item);
        modes.setAdapter(adapter);
        modes.setSelection(Modes.index(preferences.mode()));
        modes.setOnItemSelectedListener(new android.widget.AdapterView.OnItemSelectedListener() {
            @Override
            public void onItemSelected(android.widget.AdapterView<?> parent, View view, int position, long id) {
                preferences.mode(Modes.IDS[position]);
                refreshService();
            }

            @Override
            public void onNothingSelected(android.widget.AdapterView<?> parent) {}
        });
        content.addView(modes, Ui.block(this));
        Switch polish = new Switch(this);
        polish.setText(R.string.polish_enabled);
        polish.setTextColor(Ui.TEXT);
        polish.setChecked(preferences.polishEnabled());
        polish.setOnCheckedChangeListener((button, checked) -> preferences.polishEnabled(checked));
        content.addView(polish, Ui.block(this));

        content.addView(Ui.text(this, "바로 테스트", 19, true));
        EditText test = field("이 입력창을 눌러 작은 마이크를 확인해 보세요", InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_MULTI_LINE);
        test.setId(R.id.voice_test_editor);
        test.setMinLines(2);
        test.setSaveEnabled(false);
        content.addView(test, Ui.block(this));

        LinearLayout advanced = Ui.column(this, 0);
        advanced.setVisibility(View.GONE);
        addButton(content, "고급 연결 설정 (일반적으로 불필요)", () -> advanced.setVisibility(
                advanced.getVisibility() == View.VISIBLE ? View.GONE : View.VISIBLE));
        advanced.addView(Ui.text(this, "개인 PC 서버 주소", 15, true));
        server = field("HTTPS 서버 주소", InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_URI);
        server.setText(preferences.endpoint());
        advanced.addView(server, Ui.block(this));
        token = field("인증 토큰 변경할 때만 입력", InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD);
        token.setSaveEnabled(false);
        token.setImportantForAutofill(EditText.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS);
        advanced.addView(token, Ui.block(this));
        addButton(advanced, "연결 설정 저장", () -> {
            try {
                preferences.save(server.getText().toString(), token.getText().toString().trim());
                token.setText("");
                updateStatus();
                status.setText(R.string.setup_saved);
            } catch (IllegalArgumentException error) {
                status.setText(error.getMessage());
            } catch (java.security.GeneralSecurityException error) {
                status.setText("인증 정보를 저장하지 못했습니다. PC 연결 설정을 다시 받아 주세요.");
            }
        });
        content.addView(advanced);
        content.addView(Ui.text(this,
                "음성은 Tailscale을 통해 개인 PC에서 인식합니다. AI 다듬기 사용 시 전사문은 PC에서 OpenAI로 전송됩니다. PC가 켜져 있어야 합니다. 녹음·전사 이력은 저장하지 않습니다. 입력하지 못한 마지막 문장만 앱이 실행되는 동안 임시 보관하며, 닫기 또는 앱 종료 시 사라집니다.", 12, false));
        setContentView(scroll);
        scroll.requestApplyInsets();
    }

    private void openAccessibility() {
        startActivity(new Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS));
    }

    private void refreshService() {
        VoiceAccessibilityService service = VoiceAccessibilityService.active();
        if (service != null) {
            service.settingsChanged();
        }
    }

    private EditText field(String hint, int inputType) {
        EditText field = new EditText(this);
        field.setHint(hint);
        field.setTextColor(Ui.TEXT);
        field.setHintTextColor(Ui.MUTED);
        field.setTextSize(14);
        field.setInputType(inputType);
        return field;
    }

    private void addButton(LinearLayout content, String label, Runnable action) {
        Button button = Ui.button(this, label);
        button.setOnClickListener(view -> action.run());
        content.addView(button, Ui.block(this));
    }

    private void updateStatus() {
        if (connection == null) {
            return;
        }
        modeSelector.setSelection(Modes.index(preferences.mode()));
        boolean configured = false;
        try {
            configured = Endpoint.validToken(preferences.token());
        } catch (java.security.GeneralSecurityException ignored) {}
        connection.setText(probing ? "PC 서버 연결을 확인하는 중입니다…"
                : getString(configured ? R.string.connection_ready : R.string.connection_missing));
        connection.setTextColor(configured && !probing ? Ui.GREEN : Ui.MUTED);
        boolean microphone = checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED;
        boolean accessibility = VoiceAccessibilityService.active() != null;
        status.setText(getString(R.string.permission_summary,
                microphone ? "✓" : "미허용", accessibility ? "✓" : "접근성 설정 필요"));
    }

    private void importPairing() {
        try {
            if (preferences.importPairing()) {
                server.setText(preferences.endpoint());
                updateStatus();
            }
        } catch (java.security.GeneralSecurityException error) {
            status.setText(error.getMessage());
        }
    }

    @Override
    protected void onStart() {
        super.onStart();
        preferences.observe(this);
    }

    @Override
    protected void onStop() {
        preferences.unobserve(this);
        super.onStop();
    }

    @Override
    public void onSharedPreferenceChanged(android.content.SharedPreferences store, String key) {
        if ("mode".equals(key)) {
            modeSelector.setSelection(Modes.index(preferences.mode()));
        }
    }

    @Override
    protected void onResume() {
        super.onResume();
        recovery.show(RecoveryResult.ACTIVE.latest());
        updateStatus();
        importPairing();
        probeConnection();
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        recovery.show(RecoveryResult.ACTIVE.latest());
        updateStatus();
        importPairing();
        probeConnection();
    }

    private void probeConnection() {
        if (probing || !preferences.hasToken()) {
            return;
        }
        final String endpoint;
        final String credential;
        final String address;
        try {
            endpoint = preferences.endpoint();
            credential = preferences.token();
            address = preferences.serverAddress();
        } catch (java.security.GeneralSecurityException error) {
            connection.setText(R.string.connection_missing);
            return;
        }
        probing = true;
        connection.setText("PC 서버 연결을 확인하는 중입니다…");
        connection.setTextColor(Ui.MUTED);
        new Thread(() -> {
            ConnectionProbe.Result result = ConnectionProbe.check(endpoint, credential, address);
            runOnUiThread(() -> {
                probing = false;
                if (!isFinishing() && !isDestroyed()) {
                    connection.setText(result.verified ? getString(R.string.connection_verified) : result.message);
                    connection.setTextColor(result.verified ? Ui.GREEN : Ui.MUTED);
                }
            });
        }, "UltraSTTConnectionCheck").start();
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] results) {
        super.onRequestPermissionsResult(requestCode, permissions, results);
        updateStatus();
    }
}
