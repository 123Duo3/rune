use std::path::Path;
use std::sync::Arc;

use log::{debug, info};
use rinf::DartSignal;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use database::actions::analysis::analysis_audio_library;
use database::actions::metadata::scan_audio_library;
use database::actions::recommendation::sync_recommendation;
use database::connection::{MainDbConnection, RecommendationDbConnection, SearchDbConnection};

use crate::messages::library_manage::{
    ScanAudioLibraryProgress, ScanAudioLibraryRequest, ScanAudioLibraryResponse,
};
use crate::{
    AnalyseAudioLibraryProgress, AnalyseAudioLibraryRequest, AnalyseAudioLibraryResponse,
    CloseLibraryRequest, CloseLibraryResponse,
};

pub async fn close_library_request(
    lib_path: Arc<String>,
    cancel_token: Arc<CancellationToken>,
    dart_signal: DartSignal<CloseLibraryRequest>,
) {
    let request = dart_signal.message;

    info!("Closing library");

    if request.path != *lib_path {
        return;
    }

    cancel_token.cancel();

    CloseLibraryResponse {
        path: request.path.clone(),
    }
    .send_signal_to_dart()
}

pub async fn scan_audio_library_request(
    main_db: Arc<MainDbConnection>,
    search_db: Arc<Mutex<SearchDbConnection>>,
    cancel_token: Arc<CancellationToken>,
    dart_signal: DartSignal<ScanAudioLibraryRequest>,
) {
    let request = dart_signal.message;

    debug!("Scanning library summary: {:#?}", request);

    let mut search_db = search_db.lock().await;

    let file_processed = scan_audio_library(
        &main_db,
        &mut search_db,
        Path::new(&request.path),
        true,
        |progress| {
            ScanAudioLibraryProgress {
                path: request.path.clone(),
                progress: progress.try_into().unwrap(),
            }
            .send_signal_to_dart()
        },
        Some((*cancel_token).clone()),
    )
    .await
    .unwrap();

    ScanAudioLibraryResponse {
        path: request.path.clone(),
        progress: file_processed as i32,
    }
    .send_signal_to_dart()
}

pub fn determine_batch_size() -> usize {
    let num_cores = num_cpus::get();
    let batch_size = num_cores / 3 * 2;
    let min_batch_size = 1;
    let max_batch_size = 1000;

    std::cmp::min(std::cmp::max(batch_size, min_batch_size), max_batch_size)
}

pub async fn analyse_audio_library_request(
    main_db: Arc<MainDbConnection>,
    recommend_db: Arc<RecommendationDbConnection>,
    cancel_token: Arc<CancellationToken>,
    dart_signal: DartSignal<AnalyseAudioLibraryRequest>,
) {
    let request = dart_signal.message;

    debug!("Analysing media files: {:#?}", request);

    // Clone the path outside the closure
    let request_path = request.path.clone();

    // Clone the path again for use inside the closure
    let closure_request_path = request_path.clone();

    let total_files = analysis_audio_library(
        &main_db,
        Path::new(&request_path),
        determine_batch_size(),
        move |progress, total| {
            AnalyseAudioLibraryProgress {
                path: closure_request_path.clone(), // Use the cloned path here
                progress: progress.try_into().unwrap(),
                total: total.try_into().unwrap(),
            }
            .send_signal_to_dart()
        },
        Some((*cancel_token).clone()),
    )
    .await
    .expect("Audio analysis failed");

    sync_recommendation(&main_db, &recommend_db)
        .await
        .expect("Recommendation synchronization failed");

    AnalyseAudioLibraryResponse {
        path: request_path.clone(), // Use the original cloned path here
        total: total_files as i32,
    }
    .send_signal_to_dart();
}
