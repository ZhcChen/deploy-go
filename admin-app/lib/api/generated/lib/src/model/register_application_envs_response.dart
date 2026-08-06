//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'register_application_envs_response.g.dart';

/// RegisterApplicationEnvsResponse
///
/// Properties:
/// * [created]
/// * [declared]
@BuiltValue()
abstract class RegisterApplicationEnvsResponse implements Built<RegisterApplicationEnvsResponse, RegisterApplicationEnvsResponseBuilder> {
  @BuiltValueField(wireName: r'created')
  BuiltList<String> get created;

  @BuiltValueField(wireName: r'declared')
  BuiltList<String> get declared;

  RegisterApplicationEnvsResponse._();

  factory RegisterApplicationEnvsResponse([void updates(RegisterApplicationEnvsResponseBuilder b)]) = _$RegisterApplicationEnvsResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RegisterApplicationEnvsResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RegisterApplicationEnvsResponse> get serializer => _$RegisterApplicationEnvsResponseSerializer();
}

class _$RegisterApplicationEnvsResponseSerializer implements PrimitiveSerializer<RegisterApplicationEnvsResponse> {
  @override
  final Iterable<Type> types = const [RegisterApplicationEnvsResponse, _$RegisterApplicationEnvsResponse];

  @override
  final String wireName = r'RegisterApplicationEnvsResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RegisterApplicationEnvsResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'created';
    yield serializers.serialize(
      object.created,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'declared';
    yield serializers.serialize(
      object.declared,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RegisterApplicationEnvsResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RegisterApplicationEnvsResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'created':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.created.replace(valueDes);
          break;
        case r'declared':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.declared.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RegisterApplicationEnvsResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RegisterApplicationEnvsResponseBuilder();
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
