//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'set_branch_request.g.dart';

/// SetBranchRequest
///
/// Properties:
/// * [branch]
/// * [version]
@BuiltValue()
abstract class SetBranchRequest implements Built<SetBranchRequest, SetBranchRequestBuilder> {
  @BuiltValueField(wireName: r'branch')
  String get branch;

  @BuiltValueField(wireName: r'version')
  int get version;

  SetBranchRequest._();

  factory SetBranchRequest([void updates(SetBranchRequestBuilder b)]) = _$SetBranchRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SetBranchRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SetBranchRequest> get serializer => _$SetBranchRequestSerializer();
}

class _$SetBranchRequestSerializer implements PrimitiveSerializer<SetBranchRequest> {
  @override
  final Iterable<Type> types = const [SetBranchRequest, _$SetBranchRequest];

  @override
  final String wireName = r'SetBranchRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SetBranchRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'branch';
    yield serializers.serialize(
      object.branch,
      specifiedType: const FullType(String),
    );
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SetBranchRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SetBranchRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'branch':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.branch = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.version = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SetBranchRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SetBranchRequestBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
