use rand::Rng;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::io::BufReader;

const TRACKS: [&[u8]; 3] = [
    include_bytes!("../../assets/music/01.mp3"),
    include_bytes!("../../assets/music/02.mp3"),
    include_bytes!("../../assets/music/03.mp3"),
];

pub struct SoundManager {
    _stream: OutputStream,
    _handle: OutputStreamHandle,
    music_sink: Sink,
    playing: bool,
    start_index: usize,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Track {
    #[allow(dead_code)]
    None,
    Menu,
    Flight,
}

impl SoundManager {
    pub fn new() -> Option<Self> {
        let (stream, handle) = OutputStream::try_default().ok()?;
        let sink = Sink::try_new(&handle).ok()?;
        sink.set_volume(0.3);
        let start_index = rand::rng().random_range(0..TRACKS.len());
        Some(Self {
            _stream: stream,
            _handle: handle,
            music_sink: sink,
            playing: false,
            start_index,
        })
    }

    pub fn play(&mut self, _track: Track) {
        if self.playing && !self.music_sink.empty() {
            return;
        }

        if !self.playing {
            self.music_sink.stop();
            self.music_sink = Sink::try_new(&self._handle).unwrap();
            self.music_sink.set_volume(0.3);

            // Queue both tracks to play sequentially, then repeat
            self.append_playlist();
            self.music_sink.play();
            self.playing = true;
        }

        // If sink is empty, the playlist finished — restart it
        if self.music_sink.empty() {
            self.append_playlist();
        }
    }

    fn append_playlist(&self) {
        // Play all tracks starting from the random start index, wrapping around
        for i in 0..TRACKS.len() {
            let idx = (self.start_index + i) % TRACKS.len();
            let cursor = std::io::Cursor::new(TRACKS[idx]);
            let reader = BufReader::new(cursor);
            if let Ok(source) = Decoder::new(reader) {
                self.music_sink.append(source);
            }
        }
    }

    pub fn stop(&mut self) {
        self.music_sink.stop();
        self.music_sink = Sink::try_new(&self._handle).unwrap();
        self.playing = false;
    }
}
