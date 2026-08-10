// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'save_target_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SaveTargetRequest extends SaveTargetRequest {
  @override
  final String? executionMode;
  @override
  final String nodeId;
  @override
  final JsonObject? parameterSchema;
  @override
  final bool? privilegedRelease;
  @override
  final bool? privilegedReleaseConfirmed;
  @override
  final String scriptPath;
  @override
  final BuiltList<SecretFileReference>? secretFileReferences;
  @override
  final int timeoutSeconds;
  @override
  final JsonObject? verificationConfig;
  @override
  final int? version;

  factory _$SaveTargetRequest([
    void Function(SaveTargetRequestBuilder)? updates,
  ]) => (SaveTargetRequestBuilder()..update(updates))._build();

  _$SaveTargetRequest._({
    this.executionMode,
    required this.nodeId,
    this.parameterSchema,
    this.privilegedRelease,
    this.privilegedReleaseConfirmed,
    required this.scriptPath,
    this.secretFileReferences,
    required this.timeoutSeconds,
    this.verificationConfig,
    this.version,
  }) : super._();
  @override
  SaveTargetRequest rebuild(void Function(SaveTargetRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SaveTargetRequestBuilder toBuilder() =>
      SaveTargetRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SaveTargetRequest &&
        executionMode == other.executionMode &&
        nodeId == other.nodeId &&
        parameterSchema == other.parameterSchema &&
        privilegedRelease == other.privilegedRelease &&
        privilegedReleaseConfirmed == other.privilegedReleaseConfirmed &&
        scriptPath == other.scriptPath &&
        secretFileReferences == other.secretFileReferences &&
        timeoutSeconds == other.timeoutSeconds &&
        verificationConfig == other.verificationConfig &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, executionMode.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, parameterSchema.hashCode);
    _$hash = $jc(_$hash, privilegedRelease.hashCode);
    _$hash = $jc(_$hash, privilegedReleaseConfirmed.hashCode);
    _$hash = $jc(_$hash, scriptPath.hashCode);
    _$hash = $jc(_$hash, secretFileReferences.hashCode);
    _$hash = $jc(_$hash, timeoutSeconds.hashCode);
    _$hash = $jc(_$hash, verificationConfig.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SaveTargetRequest')
          ..add('executionMode', executionMode)
          ..add('nodeId', nodeId)
          ..add('parameterSchema', parameterSchema)
          ..add('privilegedRelease', privilegedRelease)
          ..add('privilegedReleaseConfirmed', privilegedReleaseConfirmed)
          ..add('scriptPath', scriptPath)
          ..add('secretFileReferences', secretFileReferences)
          ..add('timeoutSeconds', timeoutSeconds)
          ..add('verificationConfig', verificationConfig)
          ..add('version', version))
        .toString();
  }
}

class SaveTargetRequestBuilder
    implements Builder<SaveTargetRequest, SaveTargetRequestBuilder> {
  _$SaveTargetRequest? _$v;

  String? _executionMode;
  String? get executionMode => _$this._executionMode;
  set executionMode(String? executionMode) =>
      _$this._executionMode = executionMode;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  JsonObject? _parameterSchema;
  JsonObject? get parameterSchema => _$this._parameterSchema;
  set parameterSchema(JsonObject? parameterSchema) =>
      _$this._parameterSchema = parameterSchema;

  bool? _privilegedRelease;
  bool? get privilegedRelease => _$this._privilegedRelease;
  set privilegedRelease(bool? privilegedRelease) =>
      _$this._privilegedRelease = privilegedRelease;

  bool? _privilegedReleaseConfirmed;
  bool? get privilegedReleaseConfirmed => _$this._privilegedReleaseConfirmed;
  set privilegedReleaseConfirmed(bool? privilegedReleaseConfirmed) =>
      _$this._privilegedReleaseConfirmed = privilegedReleaseConfirmed;

  String? _scriptPath;
  String? get scriptPath => _$this._scriptPath;
  set scriptPath(String? scriptPath) => _$this._scriptPath = scriptPath;

  ListBuilder<SecretFileReference>? _secretFileReferences;
  ListBuilder<SecretFileReference> get secretFileReferences =>
      _$this._secretFileReferences ??= ListBuilder<SecretFileReference>();
  set secretFileReferences(
    ListBuilder<SecretFileReference>? secretFileReferences,
  ) => _$this._secretFileReferences = secretFileReferences;

  int? _timeoutSeconds;
  int? get timeoutSeconds => _$this._timeoutSeconds;
  set timeoutSeconds(int? timeoutSeconds) =>
      _$this._timeoutSeconds = timeoutSeconds;

  JsonObject? _verificationConfig;
  JsonObject? get verificationConfig => _$this._verificationConfig;
  set verificationConfig(JsonObject? verificationConfig) =>
      _$this._verificationConfig = verificationConfig;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  SaveTargetRequestBuilder() {
    SaveTargetRequest._defaults(this);
  }

  SaveTargetRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _executionMode = $v.executionMode;
      _nodeId = $v.nodeId;
      _parameterSchema = $v.parameterSchema;
      _privilegedRelease = $v.privilegedRelease;
      _privilegedReleaseConfirmed = $v.privilegedReleaseConfirmed;
      _scriptPath = $v.scriptPath;
      _secretFileReferences = $v.secretFileReferences?.toBuilder();
      _timeoutSeconds = $v.timeoutSeconds;
      _verificationConfig = $v.verificationConfig;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SaveTargetRequest other) {
    _$v = other as _$SaveTargetRequest;
  }

  @override
  void update(void Function(SaveTargetRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SaveTargetRequest build() => _build();

  _$SaveTargetRequest _build() {
    _$SaveTargetRequest _$result;
    try {
      _$result =
          _$v ??
          _$SaveTargetRequest._(
            executionMode: executionMode,
            nodeId: BuiltValueNullFieldError.checkNotNull(
              nodeId,
              r'SaveTargetRequest',
              'nodeId',
            ),
            parameterSchema: parameterSchema,
            privilegedRelease: privilegedRelease,
            privilegedReleaseConfirmed: privilegedReleaseConfirmed,
            scriptPath: BuiltValueNullFieldError.checkNotNull(
              scriptPath,
              r'SaveTargetRequest',
              'scriptPath',
            ),
            secretFileReferences: _secretFileReferences?.build(),
            timeoutSeconds: BuiltValueNullFieldError.checkNotNull(
              timeoutSeconds,
              r'SaveTargetRequest',
              'timeoutSeconds',
            ),
            verificationConfig: verificationConfig,
            version: version,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'secretFileReferences';
        _secretFileReferences?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'SaveTargetRequest',
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
