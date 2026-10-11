use std::time::Duration;

pub fn recognition(seconds: u64) -> Duration {
    Duration::from_secs(seconds.saturating_add(60).clamp(60, 900))
}

pub fn pcm_seconds(bytes: usize, sample_rate: u32) -> u64 {
    (bytes as u64).div_ceil(u64::from(sample_rate.max(1)) * 2)
}

pub fn remote(seconds: u64) -> Duration {
    recognition(seconds) + Duration::from_secs(90)
}

pub fn upload(seconds: u64) -> Duration {
    remote(seconds) + Duration::from_secs(30)
}

pub fn finalize(seconds: u64) -> Duration {
    upload(seconds) + Duration::from_secs(30)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_nine_and_ten_minute_recordings_have_nested_processing_budgets() {
        for seconds in [300, 540, 600] {
            assert_eq!(pcm_seconds(seconds as usize * 32_000, 16_000), seconds);
            assert!(recognition(seconds).as_secs() > seconds);
            assert!(remote(seconds) > recognition(seconds) + Duration::from_secs(60));
            assert!(upload(seconds) > remote(seconds));
            assert!(finalize(seconds) > upload(seconds));
        }
        assert_eq!(recognition(u64::MAX), Duration::from_secs(900));
    }

    #[tokio::test(start_paused = true)]
    async fn ten_minute_audio_can_finish_after_the_previous_two_minute_cutoff() {
        let result = tokio::time::timeout(finalize(600), async {
            tokio::time::sleep(Duration::from_secs(180)).await;
            "complete transcript"
        })
        .await;
        assert_eq!(result.unwrap(), "complete transcript");
    }
}
