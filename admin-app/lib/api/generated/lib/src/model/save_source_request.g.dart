// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'save_source_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SaveSourceRequest extends SaveSourceRequest {
  @override
  final String buildAgentId;
  @override
  final String? gitCredentialId;
  @override
  final String repositoryUrl;
  @override
  final String? sourcePolicy;
  @override
  final int? version;

  factory _$SaveSourceRequest([
    void Function(SaveSourceRequestBuilder)? updates,
  ]) => (SaveSourceRequestBuilder()..update(updates))._build();

  _$SaveSourceRequest._({
    required this.buildAgentId,
    this.gitCredentialId,
    required this.repositoryUrl,
    this.sourcePolicy,
    this.version,
  }) : super._();
  @override
  SaveSourceRequest rebuild(void Function(SaveSourceRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SaveSourceRequestBuilder toBuilder() =>
      SaveSourceRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SaveSourceRequest &&
        buildAgentId == other.buildAgentId &&
        gitCredentialId == other.gitCredentialId &&
        repositoryUrl == other.repositoryUrl &&
        sourcePolicy == other.sourcePolicy &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, buildAgentId.hashCode);
    _$hash = $jc(_$hash, gitCredentialId.hashCode);
    _$hash = $jc(_$hash, repositoryUrl.hashCode);
    _$hash = $jc(_$hash, sourcePolicy.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SaveSourceRequest')
          ..add('buildAgentId', buildAgentId)
          ..add('gitCredentialId', gitCredentialId)
          ..add('repositoryUrl', repositoryUrl)
          ..add('sourcePolicy', sourcePolicy)
          ..add('version', version))
        .toString();
  }
}

class SaveSourceRequestBuilder
    implements Builder<SaveSourceRequest, SaveSourceRequestBuilder> {
  _$SaveSourceRequest? _$v;

  String? _buildAgentId;
  String? get buildAgentId => _$this._buildAgentId;
  set buildAgentId(String? buildAgentId) => _$this._buildAgentId = buildAgentId;

  String? _gitCredentialId;
  String? get gitCredentialId => _$this._gitCredentialId;
  set gitCredentialId(String? gitCredentialId) =>
      _$this._gitCredentialId = gitCredentialId;

  String? _repositoryUrl;
  String? get repositoryUrl => _$this._repositoryUrl;
  set repositoryUrl(String? repositoryUrl) =>
      _$this._repositoryUrl = repositoryUrl;

  String? _sourcePolicy;
  String? get sourcePolicy => _$this._sourcePolicy;
  set sourcePolicy(String? sourcePolicy) => _$this._sourcePolicy = sourcePolicy;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  SaveSourceRequestBuilder() {
    SaveSourceRequest._defaults(this);
  }

  SaveSourceRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _buildAgentId = $v.buildAgentId;
      _gitCredentialId = $v.gitCredentialId;
      _repositoryUrl = $v.repositoryUrl;
      _sourcePolicy = $v.sourcePolicy;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SaveSourceRequest other) {
    _$v = other as _$SaveSourceRequest;
  }

  @override
  void update(void Function(SaveSourceRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SaveSourceRequest build() => _build();

  _$SaveSourceRequest _build() {
    final _$result =
        _$v ??
        _$SaveSourceRequest._(
          buildAgentId: BuiltValueNullFieldError.checkNotNull(
            buildAgentId,
            r'SaveSourceRequest',
            'buildAgentId',
          ),
          gitCredentialId: gitCredentialId,
          repositoryUrl: BuiltValueNullFieldError.checkNotNull(
            repositoryUrl,
            r'SaveSourceRequest',
            'repositoryUrl',
          ),
          sourcePolicy: sourcePolicy,
          version: version,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
