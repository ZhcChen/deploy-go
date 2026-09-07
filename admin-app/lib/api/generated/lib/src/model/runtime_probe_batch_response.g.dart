// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'runtime_probe_batch_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RuntimeProbeBatchResponse extends RuntimeProbeBatchResponse {
  @override
  final BuiltList<RuntimeProbeItemResponse> items;

  factory _$RuntimeProbeBatchResponse([
    void Function(RuntimeProbeBatchResponseBuilder)? updates,
  ]) => (RuntimeProbeBatchResponseBuilder()..update(updates))._build();

  _$RuntimeProbeBatchResponse._({required this.items}) : super._();
  @override
  RuntimeProbeBatchResponse rebuild(
    void Function(RuntimeProbeBatchResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  RuntimeProbeBatchResponseBuilder toBuilder() =>
      RuntimeProbeBatchResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RuntimeProbeBatchResponse && items == other.items;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, items.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'RuntimeProbeBatchResponse',
    )..add('items', items)).toString();
  }
}

class RuntimeProbeBatchResponseBuilder
    implements
        Builder<RuntimeProbeBatchResponse, RuntimeProbeBatchResponseBuilder> {
  _$RuntimeProbeBatchResponse? _$v;

  ListBuilder<RuntimeProbeItemResponse>? _items;
  ListBuilder<RuntimeProbeItemResponse> get items =>
      _$this._items ??= ListBuilder<RuntimeProbeItemResponse>();
  set items(ListBuilder<RuntimeProbeItemResponse>? items) =>
      _$this._items = items;

  RuntimeProbeBatchResponseBuilder() {
    RuntimeProbeBatchResponse._defaults(this);
  }

  RuntimeProbeBatchResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RuntimeProbeBatchResponse other) {
    _$v = other as _$RuntimeProbeBatchResponse;
  }

  @override
  void update(void Function(RuntimeProbeBatchResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RuntimeProbeBatchResponse build() => _build();

  _$RuntimeProbeBatchResponse _build() {
    _$RuntimeProbeBatchResponse _$result;
    try {
      _$result = _$v ?? _$RuntimeProbeBatchResponse._(items: items.build());
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'items';
        items.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'RuntimeProbeBatchResponse',
          _$failedField,
          e.toString(),
        );
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
