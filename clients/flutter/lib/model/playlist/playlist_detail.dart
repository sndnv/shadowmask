import 'package:shadowmask/model/playlist/playlist.dart';
import 'package:shadowmask/model/playlist/playlist_item.dart';

class PlaylistDetail {
  const PlaylistDetail({required this.playlist, required this.items});

  final Playlist playlist;
  final List<PlaylistItem> items;

  factory PlaylistDetail.fromJson(Map<String, dynamic> json) => PlaylistDetail(
    playlist: Playlist.fromJson(json),
    items: <PlaylistItem>[
      for (final dynamic item
          in (json['items'] as List<dynamic>?) ?? <dynamic>[])
        PlaylistItem.fromJson(item as Map<String, dynamic>),
    ],
  );
}
