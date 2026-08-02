// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'confirm_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ConfirmRequest extends ConfirmRequest {
  @override
  final JsonObject? parameters;
  @override
  final String snapshotHash;

  factory _$ConfirmRequest([void Function(ConfirmRequestBuilder)? updates]) =>
      (ConfirmRequestBuilder()..update(updates))._build();

  _$ConfirmRequest._({this.parameters, required this.snapshotHash}) : super._();
  @override
  ConfirmRequest rebuild(void Function(ConfirmRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ConfirmRequestBuilder toBuilder() => ConfirmRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ConfirmRequest &&
        parameters == other.parameters &&
        snapshotHash == other.snapshotHash;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, parameters.hashCode);
    _$hash = $jc(_$hash, snapshotHash.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ConfirmRequest')
          ..add('parameters', parameters)
          ..add('snapshotHash', snapshotHash))
        .toString();
  }
}

class ConfirmRequestBuilder
    implements Builder<ConfirmRequest, ConfirmRequestBuilder> {
  _$ConfirmRequest? _$v;

  JsonObject? _parameters;
  JsonObject? get parameters => _$this._parameters;
  set parameters(JsonObject? parameters) => _$this._parameters = parameters;

  String? _snapshotHash;
  String? get snapshotHash => _$this._snapshotHash;
  set snapshotHash(String? snapshotHash) => _$this._snapshotHash = snapshotHash;

  ConfirmRequestBuilder() {
    ConfirmRequest._defaults(this);
  }

  ConfirmRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _parameters = $v.parameters;
      _snapshotHash = $v.snapshotHash;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ConfirmRequest other) {
    _$v = other as _$ConfirmRequest;
  }

  @override
  void update(void Function(ConfirmRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ConfirmRequest build() => _build();

  _$ConfirmRequest _build() {
    final _$result = _$v ??
        _$ConfirmRequest._(
          parameters: parameters,
          snapshotHash: BuiltValueNullFieldError.checkNotNull(
              snapshotHash, r'ConfirmRequest', 'snapshotHash'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
