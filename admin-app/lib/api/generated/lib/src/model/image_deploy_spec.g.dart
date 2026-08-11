// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'image_deploy_spec.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ImageDeploySpec extends ImageDeploySpec {
  @override
  final BuiltList<String> envFiles;
  @override
  final int hostPort;
  @override
  final String image;
  @override
  final ImageTemplate template;

  factory _$ImageDeploySpec([void Function(ImageDeploySpecBuilder)? updates]) =>
      (ImageDeploySpecBuilder()..update(updates))._build();

  _$ImageDeploySpec._({
    required this.envFiles,
    required this.hostPort,
    required this.image,
    required this.template,
  }) : super._();
  @override
  ImageDeploySpec rebuild(void Function(ImageDeploySpecBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ImageDeploySpecBuilder toBuilder() => ImageDeploySpecBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ImageDeploySpec &&
        envFiles == other.envFiles &&
        hostPort == other.hostPort &&
        image == other.image &&
        template == other.template;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, envFiles.hashCode);
    _$hash = $jc(_$hash, hostPort.hashCode);
    _$hash = $jc(_$hash, image.hashCode);
    _$hash = $jc(_$hash, template.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ImageDeploySpec')
          ..add('envFiles', envFiles)
          ..add('hostPort', hostPort)
          ..add('image', image)
          ..add('template', template))
        .toString();
  }
}

class ImageDeploySpecBuilder
    implements Builder<ImageDeploySpec, ImageDeploySpecBuilder> {
  _$ImageDeploySpec? _$v;

  ListBuilder<String>? _envFiles;
  ListBuilder<String> get envFiles =>
      _$this._envFiles ??= ListBuilder<String>();
  set envFiles(ListBuilder<String>? envFiles) => _$this._envFiles = envFiles;

  int? _hostPort;
  int? get hostPort => _$this._hostPort;
  set hostPort(int? hostPort) => _$this._hostPort = hostPort;

  String? _image;
  String? get image => _$this._image;
  set image(String? image) => _$this._image = image;

  ImageTemplate? _template;
  ImageTemplate? get template => _$this._template;
  set template(ImageTemplate? template) => _$this._template = template;

  ImageDeploySpecBuilder() {
    ImageDeploySpec._defaults(this);
  }

  ImageDeploySpecBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _envFiles = $v.envFiles.toBuilder();
      _hostPort = $v.hostPort;
      _image = $v.image;
      _template = $v.template;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ImageDeploySpec other) {
    _$v = other as _$ImageDeploySpec;
  }

  @override
  void update(void Function(ImageDeploySpecBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ImageDeploySpec build() => _build();

  _$ImageDeploySpec _build() {
    _$ImageDeploySpec _$result;
    try {
      _$result =
          _$v ??
          _$ImageDeploySpec._(
            envFiles: envFiles.build(),
            hostPort: BuiltValueNullFieldError.checkNotNull(
              hostPort,
              r'ImageDeploySpec',
              'hostPort',
            ),
            image: BuiltValueNullFieldError.checkNotNull(
              image,
              r'ImageDeploySpec',
              'image',
            ),
            template: BuiltValueNullFieldError.checkNotNull(
              template,
              r'ImageDeploySpec',
              'template',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'envFiles';
        envFiles.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'ImageDeploySpec',
          _$failedField,
          e.toString(),
        );
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
