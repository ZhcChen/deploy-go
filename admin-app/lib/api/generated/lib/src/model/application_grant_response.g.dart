// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_grant_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationGrantResponse extends ApplicationGrantResponse {
  @override
  final String applicationId;
  @override
  final String grantedAt;

  factory _$ApplicationGrantResponse(
          [void Function(ApplicationGrantResponseBuilder)? updates]) =>
      (ApplicationGrantResponseBuilder()..update(updates))._build();

  _$ApplicationGrantResponse._(
      {required this.applicationId, required this.grantedAt})
      : super._();
  @override
  ApplicationGrantResponse rebuild(
          void Function(ApplicationGrantResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ApplicationGrantResponseBuilder toBuilder() =>
      ApplicationGrantResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationGrantResponse &&
        applicationId == other.applicationId &&
        grantedAt == other.grantedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationId.hashCode);
    _$hash = $jc(_$hash, grantedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApplicationGrantResponse')
          ..add('applicationId', applicationId)
          ..add('grantedAt', grantedAt))
        .toString();
  }
}

class ApplicationGrantResponseBuilder
    implements
        Builder<ApplicationGrantResponse, ApplicationGrantResponseBuilder> {
  _$ApplicationGrantResponse? _$v;

  String? _applicationId;
  String? get applicationId => _$this._applicationId;
  set applicationId(String? applicationId) =>
      _$this._applicationId = applicationId;

  String? _grantedAt;
  String? get grantedAt => _$this._grantedAt;
  set grantedAt(String? grantedAt) => _$this._grantedAt = grantedAt;

  ApplicationGrantResponseBuilder() {
    ApplicationGrantResponse._defaults(this);
  }

  ApplicationGrantResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationId = $v.applicationId;
      _grantedAt = $v.grantedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationGrantResponse other) {
    _$v = other as _$ApplicationGrantResponse;
  }

  @override
  void update(void Function(ApplicationGrantResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationGrantResponse build() => _build();

  _$ApplicationGrantResponse _build() {
    final _$result = _$v ??
        _$ApplicationGrantResponse._(
          applicationId: BuiltValueNullFieldError.checkNotNull(
              applicationId, r'ApplicationGrantResponse', 'applicationId'),
          grantedAt: BuiltValueNullFieldError.checkNotNull(
              grantedAt, r'ApplicationGrantResponse', 'grantedAt'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
