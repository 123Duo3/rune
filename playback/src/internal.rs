use log::{debug, error, info, warn};
use rodio::{Decoder, OutputStream, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{interval, sleep_until, Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::realtime_fft::RealTimeFFT;

#[derive(Debug)]
pub enum PlayerCommand {
    Load { index: usize },
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    Switch(usize),
    Seek(f64),
    AddToPlaylist { id: i32, path: PathBuf },
    RemoveFromPlaylist { index: usize },
    ClearPlaylist,
    MovePlayListItem { old_index: usize, new_index: usize },
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Stopped,
    Playing {
        id: i32,
        index: usize,
        path: PathBuf,
        position: Duration,
    },
    Paused {
        id: i32,
        index: usize,
        path: PathBuf,
        position: Duration,
    },
    EndOfPlaylist,
    EndOfTrack {
        id: i32,
        index: usize,
        path: PathBuf,
    },
    Error {
        id: i32,
        index: usize,
        path: PathBuf,
        error: String,
    },
    Progress {
        id: i32,
        index: usize,
        path: PathBuf,
        position: Duration,
    },
    PlaylistUpdated(Vec<i32>),
    RealtimeFFT(Vec<f32>),
}

#[derive(Debug, Clone)]
pub struct PlaylistItem {
    pub id: i32,
    pub path: PathBuf,
}

#[derive(Debug, PartialEq)]
enum InternalPlaybackState {
    Playing,
    Paused,
    Stopped,
}

pub(crate) struct PlayerInternal {
    commands: mpsc::UnboundedReceiver<PlayerCommand>,
    event_sender: mpsc::UnboundedSender<PlayerEvent>,
    realtime_fft: Arc<Mutex<RealTimeFFT>>,
    playlist: Vec<PlaylistItem>,
    current_track_id: Option<i32>,
    current_track_index: Option<usize>,
    current_track_path: Option<PathBuf>,
    sink: Option<Sink>,
    _stream: Option<OutputStream>,
    state: InternalPlaybackState,
    debounce_timer: Option<Instant>,
    cancellation_token: CancellationToken,
}

impl PlayerInternal {
    pub fn new(
        commands: mpsc::UnboundedReceiver<PlayerCommand>,
        event_sender: mpsc::UnboundedSender<PlayerEvent>,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            commands,
            event_sender,
            playlist: Vec::new(),
            current_track_id: None,
            current_track_index: None,
            current_track_path: None,
            sink: None,
            _stream: None,
            realtime_fft: Arc::new(Mutex::new(RealTimeFFT::new(512))),
            state: InternalPlaybackState::Stopped,
            debounce_timer: None,
            cancellation_token,
        }
    }

    pub async fn run(&mut self) {
        let mut progress_interval = interval(Duration::from_millis(100));

        let mut fft_receiver = self.realtime_fft.lock().unwrap().subscribe();
        loop {
            tokio::select! {
                Some(cmd) = self.commands.recv() => {
                    if self.cancellation_token.is_cancelled() {
                        debug!("Cancellation token triggered, exiting run loop");

                        self.stop();
                        break;
                    }

                    debug!("Received command: {:?}", cmd);
                    match cmd {
                        PlayerCommand::Load { index } => self.load(Some(index)),
                        PlayerCommand::Play => self.play(),
                        PlayerCommand::Pause => self.pause(),
                        PlayerCommand::Stop => self.stop(),
                        PlayerCommand::Next => self.next(),
                        PlayerCommand::Previous => self.previous(),
                        PlayerCommand::Switch(index) => self.switch(index),
                        PlayerCommand::Seek(position) => self.seek(position),
                        PlayerCommand::AddToPlaylist { id, path } => self.add_to_playlist(id, path).await,
                        PlayerCommand::RemoveFromPlaylist { index } => self.remove_from_playlist(index).await,
                        PlayerCommand::ClearPlaylist => self.clear_playlist().await,
                        PlayerCommand::MovePlayListItem {old_index, new_index} => self.move_playlist_item(old_index, new_index).await
                    }
                },
                Ok(fft_data) = fft_receiver.recv() => {
                    self.event_sender.send(PlayerEvent::RealtimeFFT(fft_data)).unwrap();
                },
                _ = progress_interval.tick() => {
                    if self.state != InternalPlaybackState::Stopped {
                        self.send_progress();
                    }
                },
                _ = async {
                    if let Some(timer) = self.debounce_timer {
                        sleep_until(timer).await;
                        true
                    } else {
                        false
                    }
                }, if self.debounce_timer.is_some() => {
                    self.debounce_timer = None;
                    self.send_playlist_updated();
                },
                _ = self.cancellation_token.cancelled() => {
                    debug!("Cancellation token triggered, exiting run loop");
                    self.stop();
                    break;
                }
            }
        }
    }

    fn load(&mut self, index: Option<usize>) {
        if let Some(index) = index {
            debug!("Loading track at index: {}", index);
            let item = &self.playlist[index];
            let file = File::open(item.path.clone());
            match file {
                Ok(file) => {
                    let source = Decoder::new(BufReader::new(file));

                    match source {
                        Ok(source) => {
                            let (stream, stream_handle) = OutputStream::try_default().unwrap();
                            let sink = Sink::try_new(&stream_handle).unwrap();
                            // Create a channel to transfer FFT data
                            let (fft_tx, mut fft_rx) = mpsc::unbounded_channel();

                            // Create a new thread for calculating realtime FFT
                            let realtime_fft = Arc::clone(&self.realtime_fft);
                            tokio::spawn(async move {
                                while let Some(data) = fft_rx.recv().await {
                                    realtime_fft.lock().unwrap().add_data(data);
                                }
                            });

                            sink.append(source.periodic_access(
                                Duration::from_millis(16),
                                move |sample| {
                                    let data: Vec<i16> =
                                        sample.take(sample.channels() as usize).collect();
                                    fft_tx.send(data).unwrap();
                                },
                            ));

                            self.sink = Some(sink);
                            self._stream = Some(stream);
                            self.current_track_index = Some(index);
                            self.current_track_id = Some(item.id);
                            self.current_track_path = Some(item.path.clone());
                            info!("Track loaded: {:?}", item.path);
                            self.event_sender
                                .send(PlayerEvent::Playing {
                                    id: self.current_track_id.unwrap(),
                                    index: self.current_track_index.unwrap(),
                                    path: self.current_track_path.clone().unwrap(),
                                    position: Duration::new(0, 0),
                                })
                                .unwrap();
                            self.state = InternalPlaybackState::Playing;
                        }
                        Err(e) => {
                            error!("Failed to decode audio: {:?}", e);
                            self.event_sender
                                .send(PlayerEvent::Error {
                                    id: self.current_track_id.unwrap(),
                                    index,
                                    path: item.path.clone(),
                                    error: "Failed to decode audio".to_string(),
                                })
                                .unwrap();
                            self.state = InternalPlaybackState::Stopped;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to open file: {:?}", e);
                    self.event_sender
                        .send(PlayerEvent::Error {
                            id: self.current_track_id.unwrap(),
                            index,
                            path: item.path.clone(),
                            error: "Failed to open file".to_string(),
                        })
                        .unwrap();
                    self.state = InternalPlaybackState::Stopped;
                }
            }
        } else {
            error!("Load command received without index");
        }
    }

    fn play(&mut self) {
        if let Some(sink) = &self.sink {
            sink.play();
            info!("Playback started");
            self.event_sender
                .send(PlayerEvent::Playing {
                    id: self.current_track_id.unwrap(),
                    index: self.current_track_index.unwrap(),
                    path: self.current_track_path.clone().unwrap(),
                    position: Duration::new(0, 0),
                })
                .unwrap();
            self.state = InternalPlaybackState::Playing;
        } else {
            info!("Loading the first track");
            self.load(Some(0));
            self.play();
        }
    }

    fn pause(&mut self) {
        if let Some(sink) = &self.sink {
            sink.pause();
            info!("Playback paused");
            self.event_sender
                .send(PlayerEvent::Paused {
                    id: self.current_track_id.unwrap(),
                    index: self.current_track_index.unwrap(),
                    path: self.current_track_path.clone().unwrap(),
                    position: sink.get_pos(),
                })
                .unwrap();
            self.state = InternalPlaybackState::Paused;
        }
    }

    fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
            info!("Playback stopped");
            self.event_sender.send(PlayerEvent::Stopped).unwrap();
            self.state = InternalPlaybackState::Stopped;
        } else {
            warn!("Stop command received but no track is loaded");
        }
    }

    fn next(&mut self) {
        if let Some(index) = self.current_track_index {
            if index + 1 < self.playlist.len() {
                self.current_track_index = Some(index + 1);
                debug!("Moving to next track: {}", index + 1);
                self.load(Some(index + 1));
            } else {
                info!("End of playlist reached");
                self.event_sender.send(PlayerEvent::EndOfPlaylist).unwrap();
                self.state = InternalPlaybackState::Stopped;
            }
        } else {
            warn!("Next command received but no track is currently playing");
        }
    }

    fn previous(&mut self) {
        if let Some(index) = self.current_track_index {
            if index > 0 {
                self.current_track_index = Some(index - 1);
                debug!("Moving to previous track: {}", index - 1);
                self.load(Some(index - 1));
            } else {
                error!("Previous command received but already at the first track");
            }
        } else {
            warn!("Previous command received but no track is currently playing");
        }
    }

    fn switch(&mut self, index: usize) {
        if index > 0 || index < self.playlist.len() {
            self.current_track_index = Some(index);
            debug!("Moving to previous track: {}", index);
            self.load(Some(index));
        } else {
            warn!("Previous command received but already at the first track");
        }
    }

    fn seek(&mut self, position: f64) {
        if let Some(sink) = &self.sink {
            match sink.try_seek(std::time::Duration::from_secs(position as u64)) {
                Ok(_) => {
                    info!("Seeking to position: {} s", position);
                    match self.event_sender.send(PlayerEvent::Playing {
                        id: self.current_track_id.unwrap(),
                        index: self.current_track_index.unwrap(),
                        path: self.current_track_path.clone().unwrap(),
                        position: sink.get_pos(),
                    }) {
                        Ok(_) => (),
                        Err(e) => error!("Failed to send Playing event: {:?}", e),
                    }
                    self.state = InternalPlaybackState::Playing;
                }
                Err(e) => error!("Failed to seek: {:?}", e),
            }
        } else {
            warn!("Seek command received but no track is loaded");
        }
    }

    async fn add_to_playlist(&mut self, id: i32, path: PathBuf) {
        debug!("Adding to playlist: {:?}", path);
        self.playlist.push(PlaylistItem { id, path });
        self.schedule_playlist_update();
    }

    async fn remove_from_playlist(&mut self, index: usize) {
        if index < self.playlist.len() {
            debug!("Removing from playlist at index: {}", index);
            self.playlist.remove(index);
            self.schedule_playlist_update();
        } else {
            error!(
                "Remove command received but index {} is out of bounds",
                index
            );
        }
    }

    async fn clear_playlist(&mut self) {
        self.playlist.clear();
        self.current_track_index = None;
        self.sink = None;
        self._stream = None;
        info!("Playlist cleared");
        self.event_sender.send(PlayerEvent::Stopped).unwrap();
        self.schedule_playlist_update();
        self.state = InternalPlaybackState::Stopped;
    }

    fn send_progress(&mut self) {
        if let Some(sink) = &self.sink {
            if sink.empty() {
                self.event_sender
                    .send(PlayerEvent::EndOfTrack {
                        id: self.current_track_id.unwrap(),
                        index: self.current_track_index.unwrap(),
                        path: self.current_track_path.clone().unwrap(),
                    })
                    .unwrap();

                if self.state != InternalPlaybackState::Stopped {
                    self.next();
                }
            } else {
                self.event_sender
                    .send(PlayerEvent::Progress {
                        id: self.current_track_id.unwrap(),
                        index: self.current_track_index.unwrap(),
                        path: self.current_track_path.clone().unwrap(),
                        position: sink.get_pos(),
                    })
                    .unwrap();
            }
        }
    }

    async fn move_playlist_item(&mut self, old_index: usize, new_index: usize) {
        if old_index >= self.playlist.len() || new_index >= self.playlist.len() {
            error!("Move command received but index is out of bounds");
            return;
        }

        if old_index == new_index {
            debug!("Move command received but old_index is the same as new_index");
            return;
        }

        debug!(
            "Moving playlist item from index {} to index {}",
            old_index, new_index
        );

        let item = self.playlist.remove(old_index);
        self.playlist.insert(new_index, item);

        // Adjust current track index if necessary
        if let Some(current_index) = self.current_track_index {
            if old_index == current_index {
                // The currently playing track was moved
                self.current_track_index = Some(new_index);
            } else if old_index < current_index && new_index >= current_index {
                // The track was moved past the current track
                self.current_track_index = Some(current_index - 1);
            } else if old_index > current_index && new_index <= current_index {
                // The track was moved before the current track
                self.current_track_index = Some(current_index + 1);
            }
        }

        self.schedule_playlist_update();
    }

    fn schedule_playlist_update(&mut self) {
        let debounce_duration = Duration::from_millis(60);
        self.debounce_timer = Some(Instant::now() + debounce_duration);
    }

    fn send_playlist_updated(&self) {
        self.event_sender
            .send(PlayerEvent::PlaylistUpdated(
                self.playlist.clone().into_iter().map(|x| x.id).collect(),
            ))
            .unwrap();
    }
}
