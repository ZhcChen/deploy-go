// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_env_file_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationEnvFileResponse extends ApplicationEnvFileResponse {
  @override
  final String applicationId;
  @override
  final String currentDigest;
  @override
  final int currentVersion;
  @override
  final String declaredAt;
  @override
  final int failedCount;
  @override
  final String fileName;
  @override
  final String format;
  @override
  final String id;
  @override
  final String module;
  @override
  final int pendingCount;
  @override
  final int succeededCount;
  @override
  final int syncingCount;
  @override
  final BuiltList<ApplicationEnvSyncResponse> syncs;
  @override
  final int targetCount;
  @override
  final String updatedAt;
  @override
  final int version;

  factory _$ApplicationEnvFileResponse([
    void Function(ApplicationEnvFileResponseBuilder)? updates,
  ]) => (ApplicationEnvFileResponseBuilder()..update(updates))._build();

  _$ApplicationEnvFileResponse._({
    required this.applicationId,
    required this.currentDigest,
    required this.currentVersion,
    required this.declaredAt,
    required this.failedCount,
    required this.fileName,
    required this.format,
    required this.id,
    required this.module,
    required this.pendingCount,
    required this.succeededCount,
    required this.syncingCount,
    required this.syncs,
    required this.targetCount,
    required this.updatedAt,
    required this.version,
  }) : super._();
  @override
  ApplicationEnvFileResponse rebuild(
    void Function(ApplicationEnvFileResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationEnvFileResponseBuilder toBuilder() =>
      ApplicationEnvFileResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationEnvFileResponse &&
        applicationId == other.applicationId &&
        currentDigest == other.currentDigest &&
        currentVersion == other.currentVersion &&
        declaredAt == other.declaredAt &&
        failedCount == other.failedCount &&
        fileName == other.fileName &&
        format == other.format &&
        id == other.id &&
        module == other.module &&
        pendingCount == other.pendingCount &&
        succeededCount == other.succeededCount &&
        syncingCount == other.syncingCount &&
        syncs == other.syncs &&
        targetCount == other.targetCount &&
        updatedAt == other.updatedAt &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationId.hashCode);
    _$hash = $jc(_$hash, currentDigest.hashCode);
    _$hash = $jc(_$hash, currentVersion.hashCode);
    _$hash = $jc(_$hash, declaredAt.hashCode);
    _$hash = $jc(_$hash, failedCount.hashCode);
    _$hash = $jc(_$hash, fileName.hashCode);
    _$hash = $jc(_$hash, format.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, module.hashCode);
    _$hash = $jc(_$hash, pendingCount.hashCode);
    _$hash = $jc(_$hash, succeededCount.hashCode);
    _$hash = $jc(_$hash, syncingCount.hashCode);
    _$hash = $jc(_$hash, syncs.hashCode);
    _$hash = $jc(_$hash, targetCount.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApplicationEnvFileResponse')
          ..add('applicationId', applicationId)
          ..add('currentDigest', currentDigest)
          ..add('currentVersion', currentVersion)
          ..add('declaredAt', declaredAt)
          ..add('failedCount', failedCount)
          ..add('fileName', fileName)
          ..add('format', format)
          ..add('id', id)
          ..add('module', module)
          ..add('pendingCount', pendingCount)
          ..add('succeededCount', succeededCount)
          ..add('syncingCount', syncingCount)
          ..add('syncs', syncs)
          ..add('targetCount', targetCount)
          ..add('updatedAt', updatedAt)
          ..add('version', version))
        .toString();
  }
}

class ApplicationEnvFileResponseBuilder
    implements
        Builder<ApplicationEnvFileResponse, ApplicationEnvFileResponseBuilder> {
  _$ApplicationEnvFileResponse? _$v;

  String? _applicationId;
  String? get applicationId => _$this._applicationId;
  set applicationId(String? applicationId) =>
      _$this._applicationId = applicationId;

  String? _currentDigest;
  String? get currentDigest => _$this._currentDigest;
  set currentDigest(String? currentDigest) =>
      _$this._currentDigest = currentDigest;

  int? _currentVersion;
  int? get currentVersion => _$this._currentVersion;
  set currentVersion(int? currentVersion) =>
      _$this._currentVersion = currentVersion;

  String? _declaredAt;
  String? get declaredAt => _$this._declaredAt;
  set declaredAt(String? declaredAt) => _$this._declaredAt = declaredAt;

  int? _failedCount;
  int? get failedCount => _$this._failedCount;
  set failedCount(int? failedCount) => _$this._failedCount = failedCount;

  String? _fileName;
  String? get fileName => _$this._fileName;
  set fileName(String? fileName) => _$this._fileName = fileName;

  String? _format;
  String? get format => _$this._format;
  set format(String? format) => _$this._format = format;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _module;
  String? get module => _$this._module;
  set module(String? module) => _$this._module = module;

  int? _pendingCount;
  int? get pendingCount => _$this._pendingCount;
  set pendingCount(int? pendingCount) => _$this._pendingCount = pendingCount;

  int? _succeededCount;
  int? get succeededCount => _$this._succeededCount;
  set succeededCount(int? succeededCount) =>
      _$this._succeededCount = succeededCount;

  int? _syncingCount;
  int? get syncingCount => _$this._syncingCount;
  set syncingCount(int? syncingCount) => _$this._syncingCount = syncingCount;

  ListBuilder<ApplicationEnvSyncResponse>? _syncs;
  ListBuilder<ApplicationEnvSyncResponse> get syncs =>
      _$this._syncs ??= ListBuilder<ApplicationEnvSyncResponse>();
  set syncs(ListBuilder<ApplicationEnvSyncResponse>? syncs) =>
      _$this._syncs = syncs;

  int? _targetCount;
  int? get targetCount => _$this._targetCount;
  set targetCount(int? targetCount) => _$this._targetCount = targetCount;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  ApplicationEnvFileResponseBuilder() {
    ApplicationEnvFileResponse._defaults(this);
  }

  ApplicationEnvFileResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationId = $v.applicationId;
      _currentDigest = $v.currentDigest;
      _currentVersion = $v.currentVersion;
      _declaredAt = $v.declaredAt;
      _failedCount = $v.failedCount;
      _fileName = $v.fileName;
      _format = $v.format;
      _id = $v.id;
      _module = $v.module;
      _pendingCount = $v.pendingCount;
      _succeededCount = $v.succeededCount;
      _syncingCount = $v.syncingCount;
      _syncs = $v.syncs.toBuilder();
      _targetCount = $v.targetCount;
      _updatedAt = $v.updatedAt;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationEnvFileResponse other) {
    _$v = other as _$ApplicationEnvFileResponse;
  }

  @override
  void update(void Function(ApplicationEnvFileResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationEnvFileResponse build() => _build();

  _$ApplicationEnvFileResponse _build() {
    _$ApplicationEnvFileResponse _$result;
    try {
      _$result =
          _$v ??
          _$ApplicationEnvFileResponse._(
            applicationId: BuiltValueNullFieldError.checkNotNull(
              applicationId,
              r'ApplicationEnvFileResponse',
              'applicationId',
            ),
            currentDigest: BuiltValueNullFieldError.checkNotNull(
              currentDigest,
              r'ApplicationEnvFileResponse',
              'currentDigest',
            ),
            currentVersion: BuiltValueNullFieldError.checkNotNull(
              currentVersion,
              r'ApplicationEnvFileResponse',
              'currentVersion',
            ),
            declaredAt: BuiltValueNullFieldError.checkNotNull(
              declaredAt,
              r'ApplicationEnvFileResponse',
              'declaredAt',
            ),
            failedCount: BuiltValueNullFieldError.checkNotNull(
              failedCount,
              r'ApplicationEnvFileResponse',
              'failedCount',
            ),
            fileName: BuiltValueNullFieldError.checkNotNull(
              fileName,
              r'ApplicationEnvFileResponse',
              'fileName',
            ),
            format: BuiltValueNullFieldError.checkNotNull(
              format,
              r'ApplicationEnvFileResponse',
              'format',
            ),
            id: BuiltValueNullFieldError.checkNotNull(
              id,
              r'ApplicationEnvFileResponse',
              'id',
            ),
            module: BuiltValueNullFieldError.checkNotNull(
              module,
              r'ApplicationEnvFileResponse',
              'module',
            ),
            pendingCount: BuiltValueNullFieldError.checkNotNull(
              pendingCount,
              r'ApplicationEnvFileResponse',
              'pendingCount',
            ),
            succeededCount: BuiltValueNullFieldError.checkNotNull(
              succeededCount,
              r'ApplicationEnvFileResponse',
              'succeededCount',
            ),
            syncingCount: BuiltValueNullFieldError.checkNotNull(
              syncingCount,
              r'ApplicationEnvFileResponse',
              'syncingCount',
            ),
            syncs: syncs.build(),
            targetCount: BuiltValueNullFieldError.checkNotNull(
              targetCount,
              r'ApplicationEnvFileResponse',
              'targetCount',
            ),
            updatedAt: BuiltValueNullFieldError.checkNotNull(
              updatedAt,
              r'ApplicationEnvFileResponse',
              'updatedAt',
            ),
            version: BuiltValueNullFieldError.checkNotNull(
              version,
              r'ApplicationEnvFileResponse',
              'version',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'syncs';
        syncs.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'ApplicationEnvFileResponse',
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
