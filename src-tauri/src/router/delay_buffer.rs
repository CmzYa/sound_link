use std::collections::VecDeque;

/// 延迟缓冲区 - 为目标设备提供音频延迟补偿
pub struct DelayBuffer {
    sample_buffer: VecDeque<f32>,   // 音频采样缓冲区
    delay_sample_count: usize,      // 延迟采样数
    channel_count: usize,           // 通道数
    max_buffer_len: usize,          // 缓冲区最大容量，防止内存无限增长
}

impl DelayBuffer {
    pub fn new(delay_ms: u32, sample_rate: u32, channel_count: usize) -> Self {
        let delay_sample_count = ((delay_ms as f64 * sample_rate as f64 / 1000.0).round() as usize
            * channel_count)
            .max(channel_count);

        // 缓冲区容量：至少2秒或延迟所需，最多10秒音频数据
        let needed_capacity = (sample_rate as usize * channel_count * 2).max(delay_sample_count * 4);
        let max_capacity = sample_rate as usize * channel_count * 10;
        let max_buffer_len = needed_capacity.min(max_capacity);

        Self {
            sample_buffer: VecDeque::with_capacity(delay_sample_count + 1000),
            delay_sample_count,
            channel_count,
            max_buffer_len,
        }
    }

    /// 批量写入音频数据，超出容量时丢弃最旧数据
    pub fn push_slice(&mut self, samples: &[f32]) {
        self.sample_buffer.extend(samples);

        // 超出最大容量时丢弃最旧的数据
        if self.sample_buffer.len() > self.max_buffer_len {
            let excess = self.sample_buffer.len() - self.max_buffer_len;
            self.sample_buffer.drain(..excess);
        }
    }

    /// 读取一帧，延迟期内返回静音
    pub fn pop_or_silent(&mut self) -> Vec<f32> {
        if self.sample_buffer.len() > self.delay_sample_count {
            let mut frame = Vec::with_capacity(self.channel_count);
            for _ in 0..self.channel_count {
                frame.push(self.sample_buffer.pop_front().unwrap_or(0.0));
            }
            frame
        } else {
            vec![0.0; self.channel_count]
        }
    }

    /// 更新延迟设置
    pub fn set_delay(&mut self, delay_ms: u32, sample_rate: u32) {
        let new_delay_sample_count = ((delay_ms as f64 * sample_rate as f64 / 1000.0).round() as usize
            * self.channel_count)
            .max(self.channel_count);
        self.delay_sample_count = new_delay_sample_count;
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.sample_buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay_buffer() {
        let mut buffer = DelayBuffer::new(100, 48000, 2);

        for i in 0..100 {
            buffer.push_slice(&[i as f32, i as f32]);
        }

        assert_eq!(buffer.sample_buffer.len(), 200);
    }
}
