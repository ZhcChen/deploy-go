// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'save_target_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SaveTargetRequest extends SaveTargetRequest {
  @override
  final String? executionMode;
  @override
  final ImageDeploySpec? imageSpec;
  @override
  final String nodeId;
  @override
  final String scriptPath;
  @override
  final BuiltList<SecretFileReference>? secretFileReferences;
  @override
  final String? targetCode;
  @override
  final int timeoutSeconds;
  @override
  final int? version;

  factory _$SaveTargetRequest([
    void Function(SaveTargetRequestBuilder)? updates,
  ]) => (SaveTargetRequestBuilder()..update(updates))._build();

  _$SaveTargetRequest._({
    this.executionMode,
    this.imageSpec,
    required this.nodeId,
    required this.scriptPath,
    this.secretFileReferences,
    this.targetCode,
    required this.timeoutSeconds,
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
        imageSpec == other.imageSpec &&
        nodeId == other.nodeId &&
        scriptPath == other.scriptPath &&
        secretFileReferences == other.secretFileReferences &&
        targetCode == other.targetCode &&
        timeoutSeconds == other.timeoutSeconds &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, executionMode.hashCode);
    _$hash = $jc(_$hash, imageSpec.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, scriptPath.hashCode);
    _$hash = $jc(_$hash, secretFileReferences.hashCode);
    _$hash = $jc(_$hash, targetCode.hashCode);
    _$hash = $jc(_$hash, timeoutSeconds.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SaveTargetRequest')
          ..add('executionMode', executionMode)
          ..add('imageSpec', imageSpec)
          ..add('nodeId', nodeId)
          ..add('scriptPath', scriptPath)
          ..add('secretFileReferences', secretFileReferences)
          ..add('targetCode', targetCode)
          ..add('timeoutSeconds', timeoutSeconds)
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

  ImageDeploySpecBuilder? _imageSpec;
  ImageDeploySpecBuilder get imageSpec =>
      _$this._imageSpec ??= ImageDeploySpecBuilder();
  set imageSpec(ImageDeploySpecBuilder? imageSpec) =>
      _$this._imageSpec = imageSpec;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  String? _scriptPath;
  String? get scriptPath => _$this._scriptPath;
  set scriptPath(String? scriptPath) => _$this._scriptPath = scriptPath;

  ListBuilder<SecretFileReference>? _secretFileReferences;
  ListBuilder<SecretFileReference> get secretFileReferences =>
      _$this._secretFileReferences ??= ListBuilder<SecretFileReference>();
  set secretFileReferences(
    ListBuilder<SecretFileReference>? secretFileReferences,
  ) => _$this._secretFileReferences = secretFileReferences;

  String? _targetCode;
  String? get targetCode => _$this._targetCode;
  set targetCode(String? targetCode) => _$this._targetCode = targetCode;

  int? _timeoutSeconds;
  int? get timeoutSeconds => _$this._timeoutSeconds;
  set timeoutSeconds(int? timeoutSeconds) =>
      _$this._timeoutSeconds = timeoutSeconds;

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
      _imageSpec = $v.imageSpec?.toBuilder();
      _nodeId = $v.nodeId;
      _scriptPath = $v.scriptPath;
      _secretFileReferences = $v.secretFileReferences?.toBuilder();
      _targetCode = $v.targetCode;
      _timeoutSeconds = $v.timeoutSeconds;
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
            imageSpec: _imageSpec?.build(),
            nodeId: BuiltValueNullFieldError.checkNotNull(
              nodeId,
              r'SaveTargetRequest',
              'nodeId',
            ),
            scriptPath: BuiltValueNullFieldError.checkNotNull(
              scriptPath,
              r'SaveTargetRequest',
              'scriptPath',
            ),
            secretFileReferences: _secretFileReferences?.build(),
            targetCode: targetCode,
            timeoutSeconds: BuiltValueNullFieldError.checkNotNull(
              timeoutSeconds,
              r'SaveTargetRequest',
              'timeoutSeconds',
            ),
            version: version,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'imageSpec';
        _imageSpec?.build();

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
