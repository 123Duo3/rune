import 'dart:async';

import 'package:fluent_ui/fluent_ui.dart';
import 'package:infinite_scroll_pagination/infinite_scroll_pagination.dart';

import '../../../widgets/track_list/track_list.dart';
import '../../../messages/media_file.pb.dart';

class TrackListView extends StatefulWidget {
  const TrackListView({super.key});

  @override
  TrackListViewState createState() => TrackListViewState();
}

class TrackListViewState extends State<TrackListView> {
  static const _pageSize = 100;

  final PagingController<int, MediaFile> _pagingController =
      PagingController(firstPageKey: 0);

  @override
  void initState() {
    super.initState();
    _pagingController.addPageRequestListener((cursor) {
      _fetchPage(cursor);
    });
  }

  Future<void> _fetchPage(int cursor) async {
    try {
      final fetchMediaFiles = FetchMediaFilesRequest(
        cursor: cursor,
        pageSize: _pageSize,
      );
      fetchMediaFiles.sendSignalToRust(); // GENERATED

      // Listen for the response from Rust
      final rustSignal = await MediaFileList.rustSignalStream.first;
      final mediaFileList = rustSignal.message;
      final newItems = mediaFileList.mediaFiles;

      final isLastPage = newItems.length < _pageSize;
      if (isLastPage) {
        _pagingController.appendLastPage(newItems);
      } else {
        final nextCursor = cursor + newItems.length;
        _pagingController.appendPage(newItems, nextCursor);
      }
    } catch (error) {
      _pagingController.error = error;
    }
  }

  @override
  Widget build(BuildContext context) {
    return TrackList(pagingController: _pagingController);
  }

  @override
  void dispose() {
    _pagingController.dispose();
    super.dispose();
  }
}
