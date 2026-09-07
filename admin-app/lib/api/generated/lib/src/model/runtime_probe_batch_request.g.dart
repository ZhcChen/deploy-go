// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'runtime_probe_batch_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RuntimeProbeBatchRequest extends RuntimeProbeBatchRequest {
  @override
  final BuiltList<String> applicationIds;

  factory _$RuntimeProbeBatchRequest([
    void Function(RuntimeProbeBatchRequestBuilder)? updates,
  ]) => (RuntimeProbeBatchRequestBuilder()..update(updates))._build();

  _$RuntimeProbeBatchRequest._({required this.applicationIds}) : super._();
  @override
  RuntimeProbeBatchRequest rebuild(
    void Function(RuntimeProbeBatchRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  RuntimeProbeBatchRequestBuilder toBuilder() =>
      RuntimeProbeBatchRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RuntimeProbeBatchRequest &&
        applicationIds == other.applicationIds;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationIds.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'RuntimeProbeBatchRequest',
    )..add('applicationIds', applicationIds)).toString();
  }
}

class RuntimeProbeBatchRequestBuilder
    implements
        Builder<RuntimeProbeBatchRequest, RuntimeProbeBatchRequestBuilder> {
  _$RuntimeProbeBatchRequest? _$v;

  ListBuilder<String>? _applicationIds;
  ListBuilder<String> get applicationIds =>
      _$this._applicationIds ??= ListBuilder<String>();
  set applicationIds(ListBuilder<String>? applicationIds) =>
      _$this._applicationIds = applicationIds;

  RuntimeProbeBatchRequestBuilder() {
    RuntimeProbeBatchRequest._defaults(this);
  }

  RuntimeProbeBatchRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationIds = $v.applicationIds.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RuntimeProbeBatchRequest other) {
    _$v = other as _$RuntimeProbeBatchRequest;
  }

  @override
  void update(void Function(RuntimeProbeBatchRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RuntimeProbeBatchRequest build() => _build();

  _$RuntimeProbeBatchRequest _build() {
    _$RuntimeProbeBatchRequest _$result;
    try {
      _$result =
          _$v ??
          _$RuntimeProbeBatchRequest._(applicationIds: applicationIds.build());
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'applicationIds';
        applicationIds.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'RuntimeProbeBatchRequest',
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
