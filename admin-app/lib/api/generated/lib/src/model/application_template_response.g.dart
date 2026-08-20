// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_template_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationTemplateResponse extends ApplicationTemplateResponse {
  @override
  final String defaultImage;
  @override
  final int defaultPort;
  @override
  final String deploymentMechanism;
  @override
  final String digest;
  @override
  final BuiltList<ApplicationTemplateFileResponse> files;
  @override
  final String id;
  @override
  final String name;
  @override
  final String summary;
  @override
  final String version;

  factory _$ApplicationTemplateResponse([
    void Function(ApplicationTemplateResponseBuilder)? updates,
  ]) => (ApplicationTemplateResponseBuilder()..update(updates))._build();

  _$ApplicationTemplateResponse._({
    required this.defaultImage,
    required this.defaultPort,
    required this.deploymentMechanism,
    required this.digest,
    required this.files,
    required this.id,
    required this.name,
    required this.summary,
    required this.version,
  }) : super._();
  @override
  ApplicationTemplateResponse rebuild(
    void Function(ApplicationTemplateResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationTemplateResponseBuilder toBuilder() =>
      ApplicationTemplateResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationTemplateResponse &&
        defaultImage == other.defaultImage &&
        defaultPort == other.defaultPort &&
        deploymentMechanism == other.deploymentMechanism &&
        digest == other.digest &&
        files == other.files &&
        id == other.id &&
        name == other.name &&
        summary == other.summary &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, defaultImage.hashCode);
    _$hash = $jc(_$hash, defaultPort.hashCode);
    _$hash = $jc(_$hash, deploymentMechanism.hashCode);
    _$hash = $jc(_$hash, digest.hashCode);
    _$hash = $jc(_$hash, files.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, summary.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApplicationTemplateResponse')
          ..add('defaultImage', defaultImage)
          ..add('defaultPort', defaultPort)
          ..add('deploymentMechanism', deploymentMechanism)
          ..add('digest', digest)
          ..add('files', files)
          ..add('id', id)
          ..add('name', name)
          ..add('summary', summary)
          ..add('version', version))
        .toString();
  }
}

class ApplicationTemplateResponseBuilder
    implements
        Builder<
          ApplicationTemplateResponse,
          ApplicationTemplateResponseBuilder
        > {
  _$ApplicationTemplateResponse? _$v;

  String? _defaultImage;
  String? get defaultImage => _$this._defaultImage;
  set defaultImage(String? defaultImage) => _$this._defaultImage = defaultImage;

  int? _defaultPort;
  int? get defaultPort => _$this._defaultPort;
  set defaultPort(int? defaultPort) => _$this._defaultPort = defaultPort;

  String? _deploymentMechanism;
  String? get deploymentMechanism => _$this._deploymentMechanism;
  set deploymentMechanism(String? deploymentMechanism) =>
      _$this._deploymentMechanism = deploymentMechanism;

  String? _digest;
  String? get digest => _$this._digest;
  set digest(String? digest) => _$this._digest = digest;

  ListBuilder<ApplicationTemplateFileResponse>? _files;
  ListBuilder<ApplicationTemplateFileResponse> get files =>
      _$this._files ??= ListBuilder<ApplicationTemplateFileResponse>();
  set files(ListBuilder<ApplicationTemplateFileResponse>? files) =>
      _$this._files = files;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _summary;
  String? get summary => _$this._summary;
  set summary(String? summary) => _$this._summary = summary;

  String? _version;
  String? get version => _$this._version;
  set version(String? version) => _$this._version = version;

  ApplicationTemplateResponseBuilder() {
    ApplicationTemplateResponse._defaults(this);
  }

  ApplicationTemplateResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _defaultImage = $v.defaultImage;
      _defaultPort = $v.defaultPort;
      _deploymentMechanism = $v.deploymentMechanism;
      _digest = $v.digest;
      _files = $v.files.toBuilder();
      _id = $v.id;
      _name = $v.name;
      _summary = $v.summary;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationTemplateResponse other) {
    _$v = other as _$ApplicationTemplateResponse;
  }

  @override
  void update(void Function(ApplicationTemplateResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationTemplateResponse build() => _build();

  _$ApplicationTemplateResponse _build() {
    _$ApplicationTemplateResponse _$result;
    try {
      _$result =
          _$v ??
          _$ApplicationTemplateResponse._(
            defaultImage: BuiltValueNullFieldError.checkNotNull(
              defaultImage,
              r'ApplicationTemplateResponse',
              'defaultImage',
            ),
            defaultPort: BuiltValueNullFieldError.checkNotNull(
              defaultPort,
              r'ApplicationTemplateResponse',
              'defaultPort',
            ),
            deploymentMechanism: BuiltValueNullFieldError.checkNotNull(
              deploymentMechanism,
              r'ApplicationTemplateResponse',
              'deploymentMechanism',
            ),
            digest: BuiltValueNullFieldError.checkNotNull(
              digest,
              r'ApplicationTemplateResponse',
              'digest',
            ),
            files: files.build(),
            id: BuiltValueNullFieldError.checkNotNull(
              id,
              r'ApplicationTemplateResponse',
              'id',
            ),
            name: BuiltValueNullFieldError.checkNotNull(
              name,
              r'ApplicationTemplateResponse',
              'name',
            ),
            summary: BuiltValueNullFieldError.checkNotNull(
              summary,
              r'ApplicationTemplateResponse',
              'summary',
            ),
            version: BuiltValueNullFieldError.checkNotNull(
              version,
              r'ApplicationTemplateResponse',
              'version',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'files';
        files.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'ApplicationTemplateResponse',
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
