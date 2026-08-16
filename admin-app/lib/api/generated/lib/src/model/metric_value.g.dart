// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'metric_value.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$MetricValue extends MetricValue {
  @override
  final String status;
  @override
  final double? value;

  factory _$MetricValue([void Function(MetricValueBuilder)? updates]) =>
      (MetricValueBuilder()..update(updates))._build();

  _$MetricValue._({required this.status, this.value}) : super._();
  @override
  MetricValue rebuild(void Function(MetricValueBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  MetricValueBuilder toBuilder() => MetricValueBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is MetricValue &&
        status == other.status &&
        value == other.value;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, value.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'MetricValue')
          ..add('status', status)
          ..add('value', value))
        .toString();
  }
}

class MetricValueBuilder implements Builder<MetricValue, MetricValueBuilder> {
  _$MetricValue? _$v;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  double? _value;
  double? get value => _$this._value;
  set value(double? value) => _$this._value = value;

  MetricValueBuilder() {
    MetricValue._defaults(this);
  }

  MetricValueBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _status = $v.status;
      _value = $v.value;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(MetricValue other) {
    _$v = other as _$MetricValue;
  }

  @override
  void update(void Function(MetricValueBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  MetricValue build() => _build();

  _$MetricValue _build() {
    final _$result =
        _$v ??
        _$MetricValue._(
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'MetricValue',
            'status',
          ),
          value: value,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
