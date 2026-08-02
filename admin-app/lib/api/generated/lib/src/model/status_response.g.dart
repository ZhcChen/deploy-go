// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'status_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$StatusResponse extends StatusResponse {
  @override
  final String status;

  factory _$StatusResponse([void Function(StatusResponseBuilder)? updates]) =>
      (StatusResponseBuilder()..update(updates))._build();

  _$StatusResponse._({required this.status}) : super._();
  @override
  StatusResponse rebuild(void Function(StatusResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  StatusResponseBuilder toBuilder() => StatusResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is StatusResponse && status == other.status;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'StatusResponse',
    )..add('status', status)).toString();
  }
}

class StatusResponseBuilder
    implements Builder<StatusResponse, StatusResponseBuilder> {
  _$StatusResponse? _$v;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  StatusResponseBuilder() {
    StatusResponse._defaults(this);
  }

  StatusResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _status = $v.status;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(StatusResponse other) {
    _$v = other as _$StatusResponse;
  }

  @override
  void update(void Function(StatusResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  StatusResponse build() => _build();

  _$StatusResponse _build() {
    final _$result =
        _$v ??
        _$StatusResponse._(
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'StatusResponse',
            'status',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
