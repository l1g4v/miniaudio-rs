use crate::base::{from_bool32, Channel, ChannelMixMode, Error, Format, MAX_CHANNELS};
use crate::frames::{Frame, Frames, Sample};
use miniaudio_sys as sys;
use std::marker::PhantomData;

#[repr(transparent)]
pub struct ChannelConverterConfig<S: Sample, Fin: Frame, Fout: Frame>(
    sys::ma_channel_converter_config,
    PhantomData<S>,
    PhantomData<Fin>,
    PhantomData<Fout>,
);

impl<S: Sample, Fin: Frame, Fout: Frame> ChannelConverterConfig<S, Fin, Fout> {
    pub fn new(
        channel_map_in: &[Channel],
        channel_map_out: &[Channel],
        mixing_mode: ChannelMixMode,
    ) -> ChannelConverterConfig<S, Fin, Fout> {
        let channel_count_in = S::channels::<Fin>();
        assert!(
            channel_count_in == channel_map_in.len(),
            "input channel count mismatch"
        );

        let channel_count_out = S::channels::<Fout>();
        assert!(
            channel_count_out == channel_map_out.len(),
            "output channel count mismatch"
        );

        ChannelConverterConfig(
            unsafe {
                sys::ma_channel_converter_config_init(
                    S::format() as _,
                    channel_count_in as _,
                    channel_map_in.as_ptr().cast(),
                    channel_count_out as _,
                    channel_map_out.as_ptr().cast(),
                    mixing_mode as _,
                )
            },
            PhantomData,
            PhantomData,
            PhantomData,
        )
    }

    #[inline]
    pub fn format(&self) -> Format {
        Format::from_c(self.0.format)
    }

    #[inline]
    pub fn channel_map_in(&self) -> &[Channel] {
        unsafe {
            std::slice::from_raw_parts(
                self.0.channelMapIn.as_ptr().cast(),
                self.0.channelsIn as usize,
            )
        }
    }

    #[inline]
    pub fn channel_map_out(&self) -> &[Channel] {
        unsafe {
            std::slice::from_raw_parts(
                self.0.channelMapOut.as_ptr().cast(),
                self.0.channelsOut as usize,
            )
        }
    }

    #[inline]
    pub fn mixing_mode(&self) -> ChannelMixMode {
        ChannelMixMode::from_c(self.0.mixingMode)
    }

    /// Returns the weight for an in/out channel mapping.
    ///
    /// These weights are only used when mixing mode is set to `ChannelMixMode::CustomWeights`.
    #[inline]
    pub fn weight(&self, channel_in_index: usize, channel_out_index: usize) -> f32 {
        // we use S::channels instead of self.0.channelsIn/Out here because it can be derived at
        // compile time and this bounds check can be eliminated.
        assert!(
            channel_in_index < S::channels::<Fin>() && channel_out_index < S::channels::<Fout>(),
            "channel in/out index out of bounds"
        );

        self.0.weights[channel_in_index][channel_out_index]
    }

    /// Set the weight for an in/out channel mapping.
    ///
    /// These weights are only used when mixing mode is set to `ChannelMixMode::CustomWeights`.
    #[inline]
    pub fn set_weight(&mut self, channel_in_index: usize, channel_out_index: usize, weight: f32) {
        // we use S::channels instead of self.0.channelsIn/Out here because it can be derived at
        // compile time and this bounds check can be eliminated.
        assert!(
            channel_in_index < S::channels::<Fin>() && channel_out_index < S::channels::<Fout>(),
            "channel in/out index out of bounds"
        );

        self.0.weights[channel_in_index][channel_out_index] = weight;
    }
}

#[repr(transparent)]
pub struct ChannelConverter<S: Sample, Fin: Frame, Fout: Frame>(
    sys::ma_channel_converter,
    PhantomData<S>,
    PhantomData<Fin>,
    PhantomData<Fout>,
);

impl<S: Sample, Fin: Frame, Fout: Frame> ChannelConverter<S, Fin, Fout> {
    pub fn new(
        config: &ChannelConverterConfig<S, Fin, Fout>,
    ) -> Result<ChannelConverter<S, Fin, Fout>, Error> {
        let mut converter = std::mem::MaybeUninit::<ChannelConverter<S, Fin, Fout>>::uninit();
        unsafe {
            Error::from_c_result(sys::ma_channel_converter_init(
                &config.0 as *const _,
                converter.as_mut_ptr().cast(),
            ))?;
            Ok(converter.assume_init())
        }
    }

    #[inline]
    pub fn format(&self) -> Format {
        Format::from_c(self.0.format)
    }

    #[inline]
    pub fn channel_map_in(&self) -> &[Channel] {
        unsafe {
            std::slice::from_raw_parts(
                self.0.channelMapIn.as_ptr().cast(),
                self.0.channelsIn as usize,
            )
        }
    }

    #[inline]
    pub fn channel_map_out(&self) -> &[Channel] {
        unsafe {
            std::slice::from_raw_parts(
                self.0.channelMapOut.as_ptr().cast(),
                self.0.channelsOut as usize,
            )
        }
    }

    #[inline]
    pub fn mixing_mode(&self) -> ChannelMixMode {
        ChannelMixMode::from_c(self.0.mixingMode)
    }

    #[inline]
    pub fn is_passthrough(&self) -> bool {
        from_bool32(self.0.isPassthrough())
    }

    #[inline]
    pub fn is_simple_shuffle(&self) -> bool {
        from_bool32(self.0.isSimpleShuffle())
    }

    #[inline]
    pub fn is_simple_mono_expansion(&self) -> bool {
        from_bool32(self.0.isSimpleMonoExpansion())
    }

    #[inline]
    pub fn is_stereo_to_mono(&self) -> bool {
        from_bool32(self.0.isStereoToMono())
    }

    #[inline]
    pub fn shuffle_table(&self) -> &[u8; MAX_CHANNELS] {
        unsafe { std::mem::transmute(&self.0.shuffleTable) }
    }

    #[inline]
    pub fn process_pcm_frames(
        &mut self,
        output: &mut Frames<S, Fout>,
        input: &Frames<S, Fin>,
    ) -> Result<(), Error> {
        if output.count() != input.count() {
            ma_debug_panic!("output and input buffers did not have the same frame count (output: {}, input: {})", output.count(), input.count());
            return Err(Error::InvalidArgs);
        }

        return Error::from_c_result(unsafe {
            sys::ma_channel_converter_process_pcm_frames(
                &mut self.0,
                output.frames_ptr_mut() as *mut _,
                input.frames_ptr() as *const _,
                output.count() as u64,
            )
        });
    }
}