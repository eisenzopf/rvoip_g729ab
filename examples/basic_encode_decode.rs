use rvoip_g729ab::{
    FrameType, G729Config, G729Decoder, G729Encoder, FRAME_SAMPLES, SPEECH_FRAME_BYTES,
};

fn main() {
    let cfg = G729Config { annex_b: false };
    let mut encoder = G729Encoder::new(cfg);
    let mut decoder = G729Decoder::new(cfg);

    let pcm_in = [0i16; FRAME_SAMPLES];
    let mut bitstream = [0u8; SPEECH_FRAME_BYTES];
    let frame_type = encoder.encode(&pcm_in, &mut bitstream);
    assert_eq!(frame_type, FrameType::Speech);

    let mut pcm_out = [0i16; FRAME_SAMPLES];
    decoder.decode(&bitstream, &mut pcm_out);
}
