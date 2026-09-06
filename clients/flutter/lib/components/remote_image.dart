import 'package:flutter/widgets.dart';

typedef RemoteImageFactory = ImageProvider Function(String url);

RemoteImageFactory remoteImage = NetworkImage.new;
