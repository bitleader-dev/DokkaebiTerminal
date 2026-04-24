// LiveKit 기반 AEC(AudioProcessingModule) 의존성 제거 이후, 모든 타겟에서 no-op 구현 사용.
// Dokkaebi 는 음성 통화/녹음 UI 부재로 echo cancellation 실사용 경로가 없다.

#[derive(Clone, Default)]
pub struct EchoCanceller;

impl EchoCanceller {
    pub fn process_reverse_stream(&mut self, _buf: &mut [i16]) {}

    pub fn process_stream(&mut self, _buf: &mut [i16]) -> anyhow::Result<()> {
        Ok(())
    }
}
