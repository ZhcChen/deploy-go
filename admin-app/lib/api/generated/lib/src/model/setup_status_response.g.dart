// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'setup_status_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SetupStatusResponse extends SetupStatusResponse {
  @override
  final bool setupEnabled;
  @override
  final bool setupRequired;

  factory _$SetupStatusResponse(
          [void Function(SetupStatusResponseBuilder)? updates]) =>
      (SetupStatusResponseBuilder()..update(updates))._build();

  _$SetupStatusResponse._(
      {required this.setupEnabled, required this.setupRequired})
      : super._();
  @override
  SetupStatusResponse rebuild(
          void Function(SetupStatusResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SetupStatusResponseBuilder toBuilder() =>
      SetupStatusResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SetupStatusResponse &&
        setupEnabled == other.setupEnabled &&
        setupRequired == other.setupRequired;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, setupEnabled.hashCode);
    _$hash = $jc(_$hash, setupRequired.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SetupStatusResponse')
          ..add('setupEnabled', setupEnabled)
          ..add('setupRequired', setupRequired))
        .toString();
  }
}

class SetupStatusResponseBuilder
    implements Builder<SetupStatusResponse, SetupStatusResponseBuilder> {
  _$SetupStatusResponse? _$v;

  bool? _setupEnabled;
  bool? get setupEnabled => _$this._setupEnabled;
  set setupEnabled(bool? setupEnabled) => _$this._setupEnabled = setupEnabled;

  bool? _setupRequired;
  bool? get setupRequired => _$this._setupRequired;
  set setupRequired(bool? setupRequired) =>
      _$this._setupRequired = setupRequired;

  SetupStatusResponseBuilder() {
    SetupStatusResponse._defaults(this);
  }

  SetupStatusResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _setupEnabled = $v.setupEnabled;
      _setupRequired = $v.setupRequired;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SetupStatusResponse other) {
    _$v = other as _$SetupStatusResponse;
  }

  @override
  void update(void Function(SetupStatusResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SetupStatusResponse build() => _build();

  _$SetupStatusResponse _build() {
    final _$result = _$v ??
        _$SetupStatusResponse._(
          setupEnabled: BuiltValueNullFieldError.checkNotNull(
              setupEnabled, r'SetupStatusResponse', 'setupEnabled'),
          setupRequired: BuiltValueNullFieldError.checkNotNull(
              setupRequired, r'SetupStatusResponse', 'setupRequired'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
