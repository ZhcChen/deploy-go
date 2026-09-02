// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'save_workspace_source_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SaveWorkspaceSourceRequest extends SaveWorkspaceSourceRequest {
  @override
  final String buildAgentId;
  @override
  final int? version;
  @override
  final String workspacePath;

  factory _$SaveWorkspaceSourceRequest([
    void Function(SaveWorkspaceSourceRequestBuilder)? updates,
  ]) => (SaveWorkspaceSourceRequestBuilder()..update(updates))._build();

  _$SaveWorkspaceSourceRequest._({
    required this.buildAgentId,
    this.version,
    required this.workspacePath,
  }) : super._();
  @override
  SaveWorkspaceSourceRequest rebuild(
    void Function(SaveWorkspaceSourceRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  SaveWorkspaceSourceRequestBuilder toBuilder() =>
      SaveWorkspaceSourceRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SaveWorkspaceSourceRequest &&
        buildAgentId == other.buildAgentId &&
        version == other.version &&
        workspacePath == other.workspacePath;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, buildAgentId.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jc(_$hash, workspacePath.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SaveWorkspaceSourceRequest')
          ..add('buildAgentId', buildAgentId)
          ..add('version', version)
          ..add('workspacePath', workspacePath))
        .toString();
  }
}

class SaveWorkspaceSourceRequestBuilder
    implements
        Builder<SaveWorkspaceSourceRequest, SaveWorkspaceSourceRequestBuilder> {
  _$SaveWorkspaceSourceRequest? _$v;

  String? _buildAgentId;
  String? get buildAgentId => _$this._buildAgentId;
  set buildAgentId(String? buildAgentId) => _$this._buildAgentId = buildAgentId;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  String? _workspacePath;
  String? get workspacePath => _$this._workspacePath;
  set workspacePath(String? workspacePath) =>
      _$this._workspacePath = workspacePath;

  SaveWorkspaceSourceRequestBuilder() {
    SaveWorkspaceSourceRequest._defaults(this);
  }

  SaveWorkspaceSourceRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _buildAgentId = $v.buildAgentId;
      _version = $v.version;
      _workspacePath = $v.workspacePath;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SaveWorkspaceSourceRequest other) {
    _$v = other as _$SaveWorkspaceSourceRequest;
  }

  @override
  void update(void Function(SaveWorkspaceSourceRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SaveWorkspaceSourceRequest build() => _build();

  _$SaveWorkspaceSourceRequest _build() {
    final _$result =
        _$v ??
        _$SaveWorkspaceSourceRequest._(
          buildAgentId: BuiltValueNullFieldError.checkNotNull(
            buildAgentId,
            r'SaveWorkspaceSourceRequest',
            'buildAgentId',
          ),
          version: version,
          workspacePath: BuiltValueNullFieldError.checkNotNull(
            workspacePath,
            r'SaveWorkspaceSourceRequest',
            'workspacePath',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
