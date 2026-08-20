// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'controlled_patch_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ControlledPatchRequest extends ControlledPatchRequest {
  @override
  final int expectedVersion;
  @override
  final String key;
  @override
  final String value;

  factory _$ControlledPatchRequest([
    void Function(ControlledPatchRequestBuilder)? updates,
  ]) => (ControlledPatchRequestBuilder()..update(updates))._build();

  _$ControlledPatchRequest._({
    required this.expectedVersion,
    required this.key,
    required this.value,
  }) : super._();
  @override
  ControlledPatchRequest rebuild(
    void Function(ControlledPatchRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ControlledPatchRequestBuilder toBuilder() =>
      ControlledPatchRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ControlledPatchRequest &&
        expectedVersion == other.expectedVersion &&
        key == other.key &&
        value == other.value;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, expectedVersion.hashCode);
    _$hash = $jc(_$hash, key.hashCode);
    _$hash = $jc(_$hash, value.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ControlledPatchRequest')
          ..add('expectedVersion', expectedVersion)
          ..add('key', key)
          ..add('value', value))
        .toString();
  }
}

class ControlledPatchRequestBuilder
    implements Builder<ControlledPatchRequest, ControlledPatchRequestBuilder> {
  _$ControlledPatchRequest? _$v;

  int? _expectedVersion;
  int? get expectedVersion => _$this._expectedVersion;
  set expectedVersion(int? expectedVersion) =>
      _$this._expectedVersion = expectedVersion;

  String? _key;
  String? get key => _$this._key;
  set key(String? key) => _$this._key = key;

  String? _value;
  String? get value => _$this._value;
  set value(String? value) => _$this._value = value;

  ControlledPatchRequestBuilder() {
    ControlledPatchRequest._defaults(this);
  }

  ControlledPatchRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _expectedVersion = $v.expectedVersion;
      _key = $v.key;
      _value = $v.value;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ControlledPatchRequest other) {
    _$v = other as _$ControlledPatchRequest;
  }

  @override
  void update(void Function(ControlledPatchRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ControlledPatchRequest build() => _build();

  _$ControlledPatchRequest _build() {
    final _$result =
        _$v ??
        _$ControlledPatchRequest._(
          expectedVersion: BuiltValueNullFieldError.checkNotNull(
            expectedVersion,
            r'ControlledPatchRequest',
            'expectedVersion',
          ),
          key: BuiltValueNullFieldError.checkNotNull(
            key,
            r'ControlledPatchRequest',
            'key',
          ),
          value: BuiltValueNullFieldError.checkNotNull(
            value,
            r'ControlledPatchRequest',
            'value',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
