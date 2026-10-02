package com.ultrastt.androidvoice;

import android.media.AudioFormat;
import android.media.AudioRecord;
import android.media.MediaRecorder;
import android.os.Handler;
import android.os.Looper;

import java.io.ByteArrayOutputStream;
import java.util.concurrent.atomic.AtomicBoolean;

final class AudioCapture implements Runnable {
    interface Listener {
        void onAudio(byte[] wav);
        void onError(String message);
        void onLevel(int percent, int seconds);
    }

    private final Handler main = new Handler(Looper.getMainLooper());
    private final Listener listener;
    private final AtomicBoolean stopped = new AtomicBoolean();
    private final AtomicBoolean cancelled = new AtomicBoolean();
    private volatile AudioRecord recorder;

    AudioCapture(Listener listener) {
        this.listener = listener;
    }

    void stop() {
        stopped.set(true);
        AudioRecord active = recorder;
        if (active != null) {
            try {
                active.stop();
            } catch (IllegalStateException ignored) {}
        }
    }

    void cancel() {
        cancelled.set(true);
        stop();
    }

    @Override
    public void run() {
        AudioRecord active = null;
        try {
            int minimum = AudioRecord.getMinBufferSize(WavCodec.SAMPLE_RATE,
                    AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT);
            if (minimum <= 0) {
                throw new IllegalStateException("이 마이크에서 16 kHz 녹음을 시작할 수 없습니다.");
            }
            active = new AudioRecord(MediaRecorder.AudioSource.VOICE_RECOGNITION,
                    WavCodec.SAMPLE_RATE, AudioFormat.CHANNEL_IN_MONO,
                    AudioFormat.ENCODING_PCM_16BIT, Math.max(minimum * 2, 6400));
            if (active.getState() != AudioRecord.STATE_INITIALIZED) {
                throw new IllegalStateException("마이크를 열지 못했습니다. 다른 녹음 앱을 닫고 다시 시도해 주세요.");
            }
            recorder = active;
            if (stopped.get()) {
                main.post(() -> {
                    if (!cancelled.get()) {
                        listener.onError("녹음이 너무 짧습니다. 한 문장을 말하고 입력해 주세요.");
                    }
                });
                return;
            }
            active.startRecording();
            if (active.getRecordingState() != AudioRecord.RECORDSTATE_RECORDING) {
                throw new IllegalStateException("녹음을 시작하지 못했습니다.");
            }
            ByteArrayOutputStream pcm = new ByteArrayOutputStream(32000);
            short[] samples = new short[1600];
            int frames = 0;
            while (!stopped.get() && frames < WavCodec.SAMPLE_RATE * WavCodec.MAX_SECONDS) {
                int count = active.read(samples, 0, samples.length, AudioRecord.READ_BLOCKING);
                if (stopped.get()) {
                    break;
                }
                if (count <= 0) {
                    throw new IllegalStateException("마이크 녹음이 중단됐습니다. 다시 시도해 주세요.");
                }
                int peak = 0;
                for (int index = 0; index < count; index++) {
                    short sample = samples[index];
                    pcm.write(sample & 0xff);
                    pcm.write((sample >> 8) & 0xff);
                    peak = Math.max(peak, Math.abs((int) sample));
                }
                frames += count;
                int level = Math.min(100, peak * 100 / 32768);
                int seconds = frames / WavCodec.SAMPLE_RATE;
                main.post(() -> {
                    if (!cancelled.get() && !stopped.get()) {
                        listener.onLevel(level, seconds);
                    }
                });
            }
            if (cancelled.get()) {
                return;
            }
            if (frames < WavCodec.SAMPLE_RATE / 5) {
                main.post(() -> {
                    if (!cancelled.get()) {
                        listener.onError("녹음이 너무 짧습니다. 한 문장을 말하고 입력해 주세요.");
                    }
                });
                return;
            }
            byte[] wav = WavCodec.encode(pcm.toByteArray());
            main.post(() -> {
                if (!cancelled.get()) {
                    listener.onAudio(wav);
                }
            });
        } catch (SecurityException error) {
            main.post(() -> {
                if (!cancelled.get()) {
                    listener.onError("마이크 권한이 없습니다. 앱 설정에서 허용해 주세요.");
                }
            });
        } catch (IllegalStateException | IllegalArgumentException error) {
            main.post(() -> {
                if (!cancelled.get()) {
                    listener.onError(error.getMessage() == null ? "마이크 녹음에 실패했습니다." : error.getMessage());
                }
            });
        } finally {
            recorder = null;
            if (active != null) {
                try {
                    active.stop();
                } catch (IllegalStateException ignored) {}
                active.release();
            }
        }
    }
}
