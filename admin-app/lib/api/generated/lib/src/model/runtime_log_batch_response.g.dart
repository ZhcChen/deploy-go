// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'runtime_log_batch_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RuntimeLogBatchResponse extends RuntimeLogBatchResponse {
  @override
  final int acknowledgedSequence;
  @override
  final String epoch;

  factory _$RuntimeLogBatchResponse([
    void Function(RuntimeLogBatchResponseBuilder)? updates,
  ]) => (RuntimeLogBatchResponseBuilder()..update(updates))._build();

  _$RuntimeLogBatchResponse._({
    required this.acknowledgedSequence,
    required this.epoch,
  }) : super._();
  @override
  RuntimeLogBatchResponse rebuild(
    void Function(RuntimeLogBatchResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  RuntimeLogBatchResponseBuilder toBuilder() =>
      RuntimeLogBatchResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RuntimeLogBatchResponse &&
        acknowledgedSequence == other.acknowledgedSequence &&
        epoch == other.epoch;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, acknowledgedSequence.hashCode);
    _$hash = $jc(_$hash, epoch.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RuntimeLogBatchResponse')
          ..add('acknowledgedSequence', acknowledgedSequence)
          ..add('epoch', epoch))
        .toString();
  }
}

class RuntimeLogBatchResponseBuilder
    implements
        Builder<RuntimeLogBatchResponse, RuntimeLogBatchResponseBuilder> {
  _$RuntimeLogBatchResponse? _$v;

  int? _acknowledgedSequence;
  int? get acknowledgedSequence => _$this._acknowledgedSequence;
  set acknowledgedSequence(int? acknowledgedSequence) =>
      _$this._acknowledgedSequence = acknowledgedSequence;

  String? _epoch;
  String? get epoch => _$this._epoch;
  set epoch(String? epoch) => _$this._epoch = epoch;

  RuntimeLogBatchResponseBuilder() {
    RuntimeLogBatchResponse._defaults(this);
  }

  RuntimeLogBatchResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _acknowledgedSequence = $v.acknowledgedSequence;
      _epoch = $v.epoch;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RuntimeLogBatchResponse other) {
    _$v = other as _$RuntimeLogBatchResponse;
  }

  @override
  void update(void Function(RuntimeLogBatchResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RuntimeLogBatchResponse build() => _build();

  _$RuntimeLogBatchResponse _build() {
    final _$result =
        _$v ??
        _$RuntimeLogBatchResponse._(
          acknowledgedSequence: BuiltValueNullFieldError.checkNotNull(
            acknowledgedSequence,
            r'RuntimeLogBatchResponse',
            'acknowledgedSequence',
          ),
          epoch: BuiltValueNullFieldError.checkNotNull(
            epoch,
            r'RuntimeLogBatchResponse',
            'epoch',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
