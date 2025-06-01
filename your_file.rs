let mut option_dict = ffmpeg::Dictionary::new();
option_dict.set("rtsp_transport", "tcp");

// Open input with options
let mut ictx = ffmpeg::format::input_with_dictionary(&self.video_info.uri, &option_dict)?;
let input = ictx
    .streams()
    .best(ffmpeg::media::Type::Video)
    .ok_or(ffmpeg::Error::StreamNotFound)?;
let video_stream_index = input.index();
let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;

let mut decoder = context_decoder.decoder().video()?;