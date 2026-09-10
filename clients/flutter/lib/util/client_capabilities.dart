export 'client_capabilities_stub.dart'
    if (dart.library.js_interop) 'client_capabilities_web.dart'
    if (dart.library.io) 'client_capabilities_native.dart';
