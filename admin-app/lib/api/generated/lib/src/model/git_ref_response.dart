//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'git_ref_response.g.dart';

/// GitRefResponse
///
/// Properties:
/// * [name]
/// * [ref]
/// * [sha]
@BuiltValue()
abstract class GitRefResponse implements Built<GitRefResponse, GitRefResponseBuilder> {
  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'ref')
  String get ref;

  @BuiltValueField(wireName: r'sha')
  String get sha;

  GitRefResponse._();

  factory GitRefResponse([void updates(GitRefResponseBuilder b)]) = _$GitRefResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(GitRefResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<GitRefResponse> get serializer => _$GitRefResponseSerializer();
}

class _$GitRefResponseSerializer implements PrimitiveSerializer<GitRefResponse> {
  @override
  final Iterable<Type> types = const [GitRefResponse, _$GitRefResponse];

  @override
  final String wireName = r'GitRefResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    GitRefResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'ref';
    yield serializers.serialize(
      object.ref,
      specifiedType: const FullType(String),
    );
    yield r'sha';
    yield serializers.serialize(
      object.sha,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    GitRefResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required GitRefResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'ref':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.ref = valueDes;
          break;
        case r'sha':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.sha = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  GitRefResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = GitRefResponseBuilder();
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
