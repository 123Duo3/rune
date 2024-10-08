import 'package:go_router/go_router.dart';

import '../routes/home.dart' as home;
import '../routes/tracks.dart' as tracks;
import '../routes/albums.dart' as albums;
import '../routes/search.dart' as search;
import '../routes/welcome.dart' as welcome;
import '../routes/artists.dart' as artists;
import '../routes/settings.dart' as settings;
import '../routes/playlists.dart' as playlists;
import '../routes/cover_wall.dart' as cover_wall;
import '../routes/query_tracks.dart' as query_tracks;
import '../routes/library_home.dart' as library_home;

final routes = <GoRoute>[
  GoRoute(
    path: '/welcome',
    builder: (context, state) => const welcome.WelcomePage(),
  ),
  GoRoute(
    path: '/welcome/scanning',
    builder: (context, state) => const welcome.ScanningPage(),
  ),
  GoRoute(
    path: '/home',
    builder: (context, state) => const home.HomePage(),
  ),
  GoRoute(
    path: '/library',
    builder: (context, state) => const library_home.LibraryHomePage(),
  ),
  GoRoute(
    path: '/artists',
    builder: (context, state) => const artists.ArtistsPage(),
  ),
  GoRoute(
    path: '/artists/:artistId',
    builder: (context, state) => query_tracks.QueryTracksPage(
      artistIds: [int.parse(state.pathParameters['artistId'] ?? "0")],
    ),
  ),
  GoRoute(
    path: '/albums',
    builder: (context, state) => const albums.AlbumsPage(),
  ),
  GoRoute(
    path: '/albums/:albumId',
    builder: (context, state) => query_tracks.QueryTracksPage(
      albumIds: [int.parse(state.pathParameters['albumId'] ?? "0")],
    ),
  ),
  GoRoute(
    path: '/playlists',
    builder: (context, state) => const playlists.PlaylistsPage(),
  ),
  GoRoute(
    path: '/playlists/:playlistsId',
    builder: (context, state) => query_tracks.QueryTracksPage(
      playlistIds: [int.parse(state.pathParameters['playlistsId'] ?? "0")],
    ),
  ),
  GoRoute(
    path: '/tracks',
    builder: (context, state) => const tracks.TracksPage(),
  ),
  GoRoute(
    path: '/settings',
    builder: (context, state) => const settings.SettingsPage(),
  ),
  GoRoute(
    path: '/search',
    builder: (context, state) => const search.SearchPage(),
  ),
  GoRoute(
    path: '/cover_wall',
    builder: (context, state) => const cover_wall.CoverWallPage(),
  ),
];