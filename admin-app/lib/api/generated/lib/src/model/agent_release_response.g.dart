// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_release_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentReleaseResponse extends AgentReleaseResponse {
  @override
  final bool active;
  @override
  final int protocolMaximum;
  @override
  final int protocolMinimum;
  @override
  final String version;

  factory _$AgentReleaseResponse([
    void Function(AgentReleaseResponseBuilder)? updates,
  ]) => (AgentReleaseResponseBuilder()..update(updates))._build();

  _$AgentReleaseResponse._({
    required this.active,
    required this.protocolMaximum,
    required this.protocolMinimum,
    required this.version,
  }) : super._();
  @override
  AgentReleaseResponse rebuild(
    void Function(AgentReleaseResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  AgentReleaseResponseBuilder toBuilder() =>
      AgentReleaseResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentReleaseResponse &&
        active == other.active &&
        protocolMaximum == other.protocolMaximum &&
        protocolMinimum == other.protocolMinimum &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, active.hashCode);
    _$hash = $jc(_$hash, protocolMaximum.hashCode);
    _$hash = $jc(_$hash, protocolMinimum.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentReleaseResponse')
          ..add('active', active)
          ..add('protocolMaximum', protocolMaximum)
          ..add('protocolMinimum', protocolMinimum)
          ..add('version', version))
        .toString();
  }
}

class AgentReleaseResponseBuilder
    implements Builder<AgentReleaseResponse, AgentReleaseResponseBuilder> {
  _$AgentReleaseResponse? _$v;

  bool? _active;
  bool? get active => _$this._active;
  set active(bool? active) => _$this._active = active;

  int? _protocolMaximum;
  int? get protocolMaximum => _$this._protocolMaximum;
  set protocolMaximum(int? protocolMaximum) =>
      _$this._protocolMaximum = protocolMaximum;

  int? _protocolMinimum;
  int? get protocolMinimum => _$this._protocolMinimum;
  set protocolMinimum(int? protocolMinimum) =>
      _$this._protocolMinimum = protocolMinimum;

  String? _version;
  String? get version => _$this._version;
  set version(String? version) => _$this._version = version;

  AgentReleaseResponseBuilder() {
    AgentReleaseResponse._defaults(this);
  }

  AgentReleaseResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _active = $v.active;
      _protocolMaximum = $v.protocolMaximum;
      _protocolMinimum = $v.protocolMinimum;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentReleaseResponse other) {
    _$v = other as _$AgentReleaseResponse;
  }

  @override
  void update(void Function(AgentReleaseResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentReleaseResponse build() => _build();

  _$AgentReleaseResponse _build() {
    final _$result =
        _$v ??
        _$AgentReleaseResponse._(
          active: BuiltValueNullFieldError.checkNotNull(
            active,
            r'AgentReleaseResponse',
            'active',
          ),
          protocolMaximum: BuiltValueNullFieldError.checkNotNull(
            protocolMaximum,
            r'AgentReleaseResponse',
            'protocolMaximum',
          ),
          protocolMinimum: BuiltValueNullFieldError.checkNotNull(
            protocolMinimum,
            r'AgentReleaseResponse',
            'protocolMinimum',
          ),
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'AgentReleaseResponse',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
