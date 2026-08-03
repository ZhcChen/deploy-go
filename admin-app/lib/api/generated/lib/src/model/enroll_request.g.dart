// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'enroll_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$EnrollRequest extends EnrollRequest {
  @override
  final String agentId;
  @override
  final String agentVersion;
  @override
  final String architecture;
  @override
  final String enrollmentToken;
  @override
  final String hostname;
  @override
  final String os;
  @override
  final int protocolVersion;

  factory _$EnrollRequest([void Function(EnrollRequestBuilder)? updates]) =>
      (EnrollRequestBuilder()..update(updates))._build();

  _$EnrollRequest._({
    required this.agentId,
    required this.agentVersion,
    required this.architecture,
    required this.enrollmentToken,
    required this.hostname,
    required this.os,
    required this.protocolVersion,
  }) : super._();
  @override
  EnrollRequest rebuild(void Function(EnrollRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  EnrollRequestBuilder toBuilder() => EnrollRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is EnrollRequest &&
        agentId == other.agentId &&
        agentVersion == other.agentVersion &&
        architecture == other.architecture &&
        enrollmentToken == other.enrollmentToken &&
        hostname == other.hostname &&
        os == other.os &&
        protocolVersion == other.protocolVersion;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, agentId.hashCode);
    _$hash = $jc(_$hash, agentVersion.hashCode);
    _$hash = $jc(_$hash, architecture.hashCode);
    _$hash = $jc(_$hash, enrollmentToken.hashCode);
    _$hash = $jc(_$hash, hostname.hashCode);
    _$hash = $jc(_$hash, os.hashCode);
    _$hash = $jc(_$hash, protocolVersion.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'EnrollRequest')
          ..add('agentId', agentId)
          ..add('agentVersion', agentVersion)
          ..add('architecture', architecture)
          ..add('enrollmentToken', enrollmentToken)
          ..add('hostname', hostname)
          ..add('os', os)
          ..add('protocolVersion', protocolVersion))
        .toString();
  }
}

class EnrollRequestBuilder
    implements Builder<EnrollRequest, EnrollRequestBuilder> {
  _$EnrollRequest? _$v;

  String? _agentId;
  String? get agentId => _$this._agentId;
  set agentId(String? agentId) => _$this._agentId = agentId;

  String? _agentVersion;
  String? get agentVersion => _$this._agentVersion;
  set agentVersion(String? agentVersion) => _$this._agentVersion = agentVersion;

  String? _architecture;
  String? get architecture => _$this._architecture;
  set architecture(String? architecture) => _$this._architecture = architecture;

  String? _enrollmentToken;
  String? get enrollmentToken => _$this._enrollmentToken;
  set enrollmentToken(String? enrollmentToken) =>
      _$this._enrollmentToken = enrollmentToken;

  String? _hostname;
  String? get hostname => _$this._hostname;
  set hostname(String? hostname) => _$this._hostname = hostname;

  String? _os;
  String? get os => _$this._os;
  set os(String? os) => _$this._os = os;

  int? _protocolVersion;
  int? get protocolVersion => _$this._protocolVersion;
  set protocolVersion(int? protocolVersion) =>
      _$this._protocolVersion = protocolVersion;

  EnrollRequestBuilder() {
    EnrollRequest._defaults(this);
  }

  EnrollRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _agentId = $v.agentId;
      _agentVersion = $v.agentVersion;
      _architecture = $v.architecture;
      _enrollmentToken = $v.enrollmentToken;
      _hostname = $v.hostname;
      _os = $v.os;
      _protocolVersion = $v.protocolVersion;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(EnrollRequest other) {
    _$v = other as _$EnrollRequest;
  }

  @override
  void update(void Function(EnrollRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  EnrollRequest build() => _build();

  _$EnrollRequest _build() {
    final _$result =
        _$v ??
        _$EnrollRequest._(
          agentId: BuiltValueNullFieldError.checkNotNull(
            agentId,
            r'EnrollRequest',
            'agentId',
          ),
          agentVersion: BuiltValueNullFieldError.checkNotNull(
            agentVersion,
            r'EnrollRequest',
            'agentVersion',
          ),
          architecture: BuiltValueNullFieldError.checkNotNull(
            architecture,
            r'EnrollRequest',
            'architecture',
          ),
          enrollmentToken: BuiltValueNullFieldError.checkNotNull(
            enrollmentToken,
            r'EnrollRequest',
            'enrollmentToken',
          ),
          hostname: BuiltValueNullFieldError.checkNotNull(
            hostname,
            r'EnrollRequest',
            'hostname',
          ),
          os: BuiltValueNullFieldError.checkNotNull(os, r'EnrollRequest', 'os'),
          protocolVersion: BuiltValueNullFieldError.checkNotNull(
            protocolVersion,
            r'EnrollRequest',
            'protocolVersion',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
