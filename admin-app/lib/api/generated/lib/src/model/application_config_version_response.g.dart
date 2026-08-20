// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_config_version_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationConfigVersionResponse
    extends ApplicationConfigVersionResponse {
  @override
  final String applicationConfigFileId;
  @override
  final int configVersion;
  @override
  final String createdAt;
  @override
  final String? createdBy;
  @override
  final String? digest;
  @override
  final String id;
  @override
  final String source_;
  @override
  final String? sourceTemplateDigest;
  @override
  final String? sourceVersionId;

  factory _$ApplicationConfigVersionResponse([
    void Function(ApplicationConfigVersionResponseBuilder)? updates,
  ]) => (ApplicationConfigVersionResponseBuilder()..update(updates))._build();

  _$ApplicationConfigVersionResponse._({
    required this.applicationConfigFileId,
    required this.configVersion,
    required this.createdAt,
    this.createdBy,
    this.digest,
    required this.id,
    required this.source_,
    this.sourceTemplateDigest,
    this.sourceVersionId,
  }) : super._();
  @override
  ApplicationConfigVersionResponse rebuild(
    void Function(ApplicationConfigVersionResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationConfigVersionResponseBuilder toBuilder() =>
      ApplicationConfigVersionResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationConfigVersionResponse &&
        applicationConfigFileId == other.applicationConfigFileId &&
        configVersion == other.configVersion &&
        createdAt == other.createdAt &&
        createdBy == other.createdBy &&
        digest == other.digest &&
        id == other.id &&
        source_ == other.source_ &&
        sourceTemplateDigest == other.sourceTemplateDigest &&
        sourceVersionId == other.sourceVersionId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationConfigFileId.hashCode);
    _$hash = $jc(_$hash, configVersion.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, createdBy.hashCode);
    _$hash = $jc(_$hash, digest.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, source_.hashCode);
    _$hash = $jc(_$hash, sourceTemplateDigest.hashCode);
    _$hash = $jc(_$hash, sourceVersionId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApplicationConfigVersionResponse')
          ..add('applicationConfigFileId', applicationConfigFileId)
          ..add('configVersion', configVersion)
          ..add('createdAt', createdAt)
          ..add('createdBy', createdBy)
          ..add('digest', digest)
          ..add('id', id)
          ..add('source_', source_)
          ..add('sourceTemplateDigest', sourceTemplateDigest)
          ..add('sourceVersionId', sourceVersionId))
        .toString();
  }
}

class ApplicationConfigVersionResponseBuilder
    implements
        Builder<
          ApplicationConfigVersionResponse,
          ApplicationConfigVersionResponseBuilder
        > {
  _$ApplicationConfigVersionResponse? _$v;

  String? _applicationConfigFileId;
  String? get applicationConfigFileId => _$this._applicationConfigFileId;
  set applicationConfigFileId(String? applicationConfigFileId) =>
      _$this._applicationConfigFileId = applicationConfigFileId;

  int? _configVersion;
  int? get configVersion => _$this._configVersion;
  set configVersion(int? configVersion) =>
      _$this._configVersion = configVersion;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _createdBy;
  String? get createdBy => _$this._createdBy;
  set createdBy(String? createdBy) => _$this._createdBy = createdBy;

  String? _digest;
  String? get digest => _$this._digest;
  set digest(String? digest) => _$this._digest = digest;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _source_;
  String? get source_ => _$this._source_;
  set source_(String? source_) => _$this._source_ = source_;

  String? _sourceTemplateDigest;
  String? get sourceTemplateDigest => _$this._sourceTemplateDigest;
  set sourceTemplateDigest(String? sourceTemplateDigest) =>
      _$this._sourceTemplateDigest = sourceTemplateDigest;

  String? _sourceVersionId;
  String? get sourceVersionId => _$this._sourceVersionId;
  set sourceVersionId(String? sourceVersionId) =>
      _$this._sourceVersionId = sourceVersionId;

  ApplicationConfigVersionResponseBuilder() {
    ApplicationConfigVersionResponse._defaults(this);
  }

  ApplicationConfigVersionResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationConfigFileId = $v.applicationConfigFileId;
      _configVersion = $v.configVersion;
      _createdAt = $v.createdAt;
      _createdBy = $v.createdBy;
      _digest = $v.digest;
      _id = $v.id;
      _source_ = $v.source_;
      _sourceTemplateDigest = $v.sourceTemplateDigest;
      _sourceVersionId = $v.sourceVersionId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationConfigVersionResponse other) {
    _$v = other as _$ApplicationConfigVersionResponse;
  }

  @override
  void update(void Function(ApplicationConfigVersionResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationConfigVersionResponse build() => _build();

  _$ApplicationConfigVersionResponse _build() {
    final _$result =
        _$v ??
        _$ApplicationConfigVersionResponse._(
          applicationConfigFileId: BuiltValueNullFieldError.checkNotNull(
            applicationConfigFileId,
            r'ApplicationConfigVersionResponse',
            'applicationConfigFileId',
          ),
          configVersion: BuiltValueNullFieldError.checkNotNull(
            configVersion,
            r'ApplicationConfigVersionResponse',
            'configVersion',
          ),
          createdAt: BuiltValueNullFieldError.checkNotNull(
            createdAt,
            r'ApplicationConfigVersionResponse',
            'createdAt',
          ),
          createdBy: createdBy,
          digest: digest,
          id: BuiltValueNullFieldError.checkNotNull(
            id,
            r'ApplicationConfigVersionResponse',
            'id',
          ),
          source_: BuiltValueNullFieldError.checkNotNull(
            source_,
            r'ApplicationConfigVersionResponse',
            'source_',
          ),
          sourceTemplateDigest: sourceTemplateDigest,
          sourceVersionId: sourceVersionId,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
