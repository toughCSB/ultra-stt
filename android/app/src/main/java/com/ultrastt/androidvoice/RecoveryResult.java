package com.ultrastt.androidvoice;

/** The last undelivered transcription, kept only for this app process. */
final class RecoveryResult {
    static final RecoveryResult ACTIVE = new RecoveryResult();

    static final class Snapshot {
        final long id;
        final String text;

        Snapshot(long id, String text) {
            this.id = id;
            this.text = text;
        }
    }

    private long nextId;
    private Snapshot latest;

    synchronized Snapshot saveIfCurrent(EditorGuard guard, long session, String text) {
        if (!guard.accepts(session) || text == null || text.isEmpty()) {
            return null;
        }
        latest = new Snapshot(++nextId, text);
        return latest;
    }

    synchronized Snapshot latest() {
        return latest;
    }

    synchronized void clear(long id) {
        if (latest != null && latest.id == id) {
            latest = null;
        }
    }

    private RecoveryResult() {}
}
