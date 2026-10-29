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
import android.widget.RadioButton;
import android.widget.RadioGroup;
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
    private ApiKeyPanel apiKeyPanel;
    private TextView apiConnectionStatus;
    private Button apiConnectionTest;
    private int apiConnectionRevision;
    private LinearLayout legacySection;
    private LinearLayout directSection;
    private RadioGroup engineSelector;
    private RadioButton legacyChoice;
    private RadioButton directChoice;
    private TextView engineStatus;
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
                "입력창 위에 반투명 마이크가 나타납니다. 짧게 누르면 녹음, 누른 뒤 바로 움직이면 위치 이동입니다. 가만히 길게 누르면 진동 후 반원 모드 메뉴가 열립니다. 원하는 아이콘으로 밀고 손을 떼세요. 이동한 위치는 기억합니다.", 14, false));
        addButton(content, "마이크 위치 초기화", () -> {
            preferences.resetBubblePosition();
            refreshService();
            android.widget.Toast.makeText(this, "마이크를 입력창 위 기본 위치로 되돌렸습니다.", android.widget.Toast.LENGTH_SHORT).show();
        });
        content.addView(Ui.text(this, "엔진 선택", 19, true));
        engineStatus = Ui.text(this, "", 15, true);
        engineStatus.setAccessibilityLiveRegion(View.ACCESSIBILITY_LIVE_REGION_POLITE);
        content.addView(engineStatus);
        engineSelector = new RadioGroup(this);
        legacyChoice = engineChoice("기존 엔진\nSB의 Whisper → Codex CLI\n기존 연결·구독 사용", Ui.GREEN);
        directChoice = engineChoice("OpenAI 직접 연결\nGPT Transcribe → GPT-6 Luna\nPC 없이 사용 · API 사용량 별도 과금", android.graphics.Color.rgb(56, 189, 248));
        engineSelector.addView(legacyChoice, Ui.block(this));
        engineSelector.addView(directChoice, Ui.block(this));
        engineSelector.check(Engine.OPENAI.equals(preferences.engine()) ? directChoice.getId() : legacyChoice.getId());
        engineSelector.setOnCheckedChangeListener((group, checked) -> {
            preferences.engine(checked == directChoice.getId() ? Engine.OPENAI : Engine.LEGACY);
            updateEngineSections();
            refreshService();
        });
        content.addView(engineSelector, Ui.block(this));
        content.addView(Ui.text(this,
                "카드를 누르면 저장됩니다. 다음 녹음부터 적용되며, 기존 서버 연결과 API 키는 서로 덮어쓰지 않습니다.", 13, false));

        legacySection = Ui.column(this, 0);
        connection = Ui.text(this, "", 15, true);
        legacySection.addView(connection);
        addButton(legacySection, "PC 서버 연결 확인", this::probeConnection);
        content.addView(legacySection);

        directSection = Ui.column(this, 0);
        apiConnectionStatus = Ui.text(this, "", 14, true);
        apiConnectionStatus.setAccessibilityLiveRegion(View.ACCESSIBILITY_LIVE_REGION_POLITE);
        apiKeyPanel = new ApiKeyPanel(this, new ApiKeyPanel.Store() {
            public boolean hasKey() { return preferences.hasOpenAiKey(); }
            public void save(String key) throws java.security.GeneralSecurityException {
                preferences.saveOpenAiKey(key);
                resetApiConnectionTest();
            }
            public void clear() throws java.security.GeneralSecurityException {
                preferences.clearOpenAiKey();
                resetApiConnectionTest();
            }
        });
        directSection.addView(apiKeyPanel, Ui.block(this));
        apiConnectionTest = Ui.button(this, "OpenAI 연결 테스트");
        apiConnectionTest.setOnClickListener(view -> probeOpenAi());
        directSection.addView(apiConnectionTest, Ui.block(this));
        directSection.addView(apiConnectionStatus);
        resetApiConnectionTest();
        directSection.addView(Ui.text(this,
                "녹음이 끝난 뒤 음성을 OpenAI에 직접 보내고, 다듬기 사용 시 전사문도 OpenAI로 보냅니다. PC 서버와 Tailscale은 사용하지 않습니다. 요청마다 사용량이 발생할 수 있습니다. 키 저장·엔진 변경만으로는 API 요청을 보내지 않습니다.", 12, false));
        content.addView(directSection);
        status = Ui.text(this, "", 13, false);
        content.addView(status);
        updateEngineSections();

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
        legacySection.addView(advanced);
        legacySection.addView(Ui.text(this,
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
        updateEngineSections();
        apiKeyPanel.refresh();
        boolean configured = false;
        if (Engine.LEGACY.equals(preferences.engine())) {
            try {
                configured = Endpoint.validToken(preferences.token());
            } catch (java.security.GeneralSecurityException ignored) {}
        }
        connection.setText(probing ? "PC 서버 연결을 확인하는 중입니다…"
                : getString(configured ? R.string.connection_ready : R.string.connection_missing));
        connection.setTextColor(configured && !probing ? Ui.GREEN : Ui.MUTED);
        boolean microphone = checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED;
        boolean accessibility = VoiceAccessibilityService.active() != null;
        status.setText(getString(R.string.permission_summary,
                microphone ? "✓" : "미허용", accessibility ? "✓" : "접근성 설정 필요"));
    }

    private void updateEngineSections() {
        boolean direct = Engine.OPENAI.equals(preferences.engine());
        int selectedId = direct ? directChoice.getId() : legacyChoice.getId();
        if (engineSelector.getCheckedRadioButtonId() != selectedId) {
            engineSelector.check(selectedId);
        }
        legacyChoice.setBackground(Ui.background(this, Ui.PANEL, !direct));
        android.graphics.drawable.GradientDrawable directBackground = Ui.background(this,
                android.graphics.Color.rgb(12, 34, 66), false);
        directBackground.setStroke(Ui.dp(this, direct ? 2 : 1), direct
                ? android.graphics.Color.rgb(56, 189, 248) : android.graphics.Color.rgb(35, 67, 80));
        directChoice.setBackground(directBackground);
        engineStatus.setText(direct ? "현재 선택: OpenAI 직접 연결 · 다음 녹음에 적용"
                : "현재 선택: 기존 엔진 · 다음 녹음에 적용");
        legacySection.setVisibility(direct ? View.GONE : View.VISIBLE);
        directSection.setVisibility(direct ? View.VISIBLE : View.GONE);
    }

    private RadioButton engineChoice(String label, int color) {
        RadioButton choice = new RadioButton(this);
        choice.setId(View.generateViewId());
        choice.setText(label);
        choice.setTextColor(Ui.TEXT);
        choice.setTextSize(16);
        choice.setMinHeight(Ui.dp(this, 104));
        choice.setPadding(Ui.dp(this, 12), Ui.dp(this, 14), Ui.dp(this, 12), Ui.dp(this, 14));
        choice.setButtonTintList(android.content.res.ColorStateList.valueOf(color));
        return choice;
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
        } else if ("engine".equals(key)) {
            resetApiConnectionTest();
            updateEngineSections();
            updateStatus();
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

    private void resetApiConnectionTest() {
        apiConnectionRevision++;
        if (apiConnectionTest != null) apiConnectionTest.setEnabled(preferences.hasOpenAiKey());
        if (apiConnectionStatus != null) {
            apiConnectionStatus.setText("연결 미확인 · 저장한 키로 모델 조회만 확인합니다. 음성을 보내지 않습니다.");
            apiConnectionStatus.setTextColor(Ui.MUTED);
        }
    }

    private void probeOpenAi() {
        if (!apiConnectionTest.isEnabled()) return;
        final int expected = ++apiConnectionRevision;
        apiConnectionTest.setEnabled(false);
        apiConnectionStatus.setText("OpenAI 연결 확인 중…");
        apiConnectionStatus.setTextColor(Ui.MUTED);
        new Thread(() -> {
            ConnectionProbe.Result checked;
            try {
                checked = DirectOpenAi.checkConnection(HttpTransport.openAiClient(), preferences.openAiKey());
            } catch (java.security.GeneralSecurityException failure) {
                checked = new ConnectionProbe.Result(false, "저장된 API 키를 읽지 못했습니다. 키 수정에서 다시 저장해 주세요.");
            }
            final ConnectionProbe.Result result = checked;
            runOnUiThread(() -> {
                if (isFinishing() || isDestroyed() || expected != apiConnectionRevision) return;
                apiConnectionTest.setEnabled(preferences.hasOpenAiKey());
                apiConnectionStatus.setText(result.message);
                apiConnectionStatus.setTextColor(result.verified ? Ui.GREEN : android.graphics.Color.rgb(255, 139, 139));
            });
        }, "UltraSTTOpenAiConnectionCheck").start();
    }

    private void probeConnection() {
        if (!Engine.LEGACY.equals(preferences.engine()) || probing || !preferences.hasToken()) {
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
